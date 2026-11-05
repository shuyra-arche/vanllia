#!/usr/bin/env nu

# cargo QEMU runner — riscv64gc-unknown-none-elf
#
# 用法：`.cargo/config.toml` 的 `target.<triple>.runner` 指向本脚本，cargo 把本次要跑的 ELF
#   追加为位置参数。Kernel 不是 Source workspace 成员，显式从 Source 根跑：
#     SHUYRA_INITRD=<absolute-initrd> cargo run \
#       --manifest-path components/kernel/Cargo.toml --release
#   或 `SHUYRA_INITRD=<absolute-initrd> cargo shuyra --release`。镜像选择见
#   `qemu-args.nu` 的 `chosen-initrd`；不传时回退到 ELF 同目录的 `initrd.img`。
#
# 只做两件事：
#   1) **cargo 集成点**：接住 cargo 给的 ELF，转交 `scripts/boot.nu`（**qemu 起法的唯一出处**）。
#   2) **交互式跑的终端流与证据归档**。
#   判定**今天哪也不在**（验收门已迁到 `embedded-test`）。本脚本结尾只打**一行观察**
#   （退出码 + 正常自退 / 检出内核 panic / 被 host 超时杀 / 非零退出）：任何情况都正常结束、
#   **不 exit 1**，故 `cargo run` 的退出码不因内核行为而变。
#
# 证据（都落在 <TRACE_OUT>；qemu 在其中启动 ⇒ 导出与控制台捕获就地落盘）：
#   diagnose-<seed>-<ts>.jsonl     结构化导出 sqware-diagnose.jsonl，**恒归档**
#   console-<seed>-<ts>.log        终端捕获，只有「该轮看起来正常」时才删，否则归档
#   console-<seed>-<ts>-stale.log  起跑前发现的上一轮残留捕获：先归档，**绝不删**
#   「看起来正常」= qemu 退出码 0 且捕获里无 `[panic] at`。
#
# 可配置环境变量:
#   TRACE_OUT     归档目录（默认 <project>/trace）
#   QEMU_SEED     传给 boot.nu；本脚本未设时自行生成一个并打印（SEED 行）
#   其余（QEMU_TIMEOUT / QEMU_MEM / QEMU_SMP / QEMU_ICOUNT / QEMU_SEMI / QEMU_FEATURES /
#   QEMU_GDB / QEMU_EXTRA_ARGS）由 scripts/boot.nu 解释——见该文件头注。
#
# 已知语义地雷（改本脚本前先读）：
#   - nu 脚本在**外部命令非零退出时当场中止整个脚本**（该语句之后的语句、以及调用方其后的
#     语句都不执行）⇒ 调用必须包 `try … catch { }`，否则被 timeout 杀掉时归档永不执行
#     （旧版 runner 的归档分支正是因此从未跑过）。
#   - 用 `complete` 捕获命令自身的退出码；不从 `try` 块外读 LAST_EXIT_CODE。
#     `tee { print --raw --no-newline }` 同时把原始字节流实时交给终端。
#   - `o+e>|` 把 stderr 并进捕获（qemu 那行 `terminating on signal … (/usr/bin/timeout)` 走
#     stderr，不并进来则捕获里没有它——曾据此写过一句永不成立的判定）。
#   - **不要**用 `with-env` 包 qemu 调用：它连 `LAST_EXIT_CODE` 一起还原，退出码就丢了。

use ./qemu-args.nu [chosen-initrd chosen-sbi]

def main [elf: path] {
  # **造镜像那一步在编内核之外**（`crates/image`，用户裁定"initrd 与 kernel 何干"）：
  # `cargo run` 只管编内核 ⇒ 这里查镜像在不在，不在就报一句清楚的话，
  # **而不是静默起一台没有程序的机器**（那正是最难查的一类假象）。
  # One selector: explicit `SHUYRA_INITRD`, else legacy ELF-adjacent fallback.
  let initrd = (chosen-initrd $elf)
  if ($initrd == null) {
    error make {msg: $"没有镜像：SHUYRA_INITRD 未设置，且 ELF 旁没有 initrd.img
  先造它：cargo image build product <档>（或 nu scripts/build.nu <档>），再设 SHUYRA_INITRD=<打印的绝对路径>。默认 product release。"}
  }
  # Pass the same absolute choice through the trace-directory cd.
  $env.SHUYRA_INITRD = ($initrd | into string)
  # Freeze the firmware before cd: preserve `default`, resolve custom paths.
  let sbi = (chosen-sbi)
  $env.SHUYRA_SBI = ($sbi | into string)
  let cfg = config $elf

  print $"SEED: ($cfg.seed)"
  let code = (run_qemu $cfg)
  archive $cfg $code
}

def config [elf: path] {
  let proj_root = ($env.FILE_PWD | path dirname)
  let trapdir = ($env.TRACE_OUT? | default ($proj_root | path join "trace"))
  mkdir $trapdir
  {
    proj_root: $proj_root
    elf: ($elf | path expand)
    trapdir: $trapdir
    seed: ($env.QEMU_SEED? | default ((random binary 4 | into int) | into string))
  }
}

def run_qemu [cfg: record] {
  # 起 qemu 前 cd 进归档目录：导出文件与 console 捕获都就地落盘，消除对调用者 CWD 的隐式
  # 依赖（-kernel 是绝对路径；-bios 是 default 或已校验的绝对路径，cd 无损）。
  cd $cfg.trapdir
  rm --force sqware-diagnose.jsonl   # 干净基线（guest create 本会 truncate，双保险）
  # 上一轮的残留捕获：先归档再起跑，**不删**——终端捕获是「挂在哪一步」的唯一直接证据。
  rescue
  let cap = $"sqware-($cfg.seed).cap"
  # stdin 直接继承（交互式输入进 guest 控制台）；qemu 起法归 scripts/boot.nu。
  $env.QEMU_SEED = $cfg.seed
  let boot = ($cfg.proj_root | path join "scripts" "boot.nu")
  # Keep live output while collecting the actual boot process status. Reading
  # LAST_EXIT_CODE outside a try scope can mislabel a failed boot as normal.
  let result = (^nu $boot $cfg.elf o+e>| tee { save --force $cap }
    | tee { print --raw --no-newline } | complete)
  $result.exit_code
}

# 上一轮未收尾的终端捕获 → 归档（不删除）。
def rescue [] {
  let ts = (date now | format date '%Y%m%d-%H%M%S')
  for f in (glob "sqware-*.cap") {
      let seed = (($f | path basename) | str replace --regex '^sqware-' '' | str replace --regex '\.cap$' '')
      let dump = $"console-($seed)-($ts)-stale.log"
      mv $f $dump
      print $"上轮未收尾的捕获 -> ($dump)"
  }
}

def archive [cfg: record, code: int] {
  cd $cfg.trapdir
  let ts = (date now | format date '%Y%m%d-%H%M%S')
  let export = "sqware-diagnose.jsonl"
  let cap = $"sqware-($cfg.seed).cap"

  # 两个观测量（都不是判据）：qemu 退出码（run_qemu 已返回）+ 捕获里的内核 panic 报告头。
  let text = if ($cap | path exists) { open $cap --raw } else { "" }
  let panicked = ($text | str contains "[panic] at")
  # 「看起来正常」= 自退（退出码 0）且无 panic —— 只有这一档才敢丢终端捕获。
  let normal = ($code == 0) and (not $panicked)

  # 1) 结构化导出：**恒归档**（无论成败）——复盘靠它。
  if ($export | path exists) {
      let dumped = $"diagnose-($cfg.seed)-($ts).jsonl"
      mv $export $dumped
      print $"诊断导出 -> ($cfg.trapdir)/($dumped)"
  }

  # 2) 终端捕获：只有「看起来正常」的一轮才丢；其余一律归档。
  if ($cap | path exists) {
      if $normal {
          rm --force $cap
      } else {
          let dump = $"console-($cfg.seed)-($ts).log"
          mv $cap $dump
          print $"捕获归档 -> ($cfg.trapdir)/($dump)"
      }
  }

  # 3) 一行观察（信息，不是判据）：不 exit 1，`cargo run` 的退出码不因内核行为而变。
  #    panic 先于 124：panic 若走到 halt_loop 兜底自旋，必然被 host 超时杀，根因是 panic。
  let cause = if $normal {
      "正常自退"
  } else if $panicked {
      "检出内核 panic"
  } else if $code == 124 {
      "被 host 超时杀"
  } else {
      "非零退出（非超时、无 panic）"
  }
  print $"观察：qemu 退出码 ($code) · ($cause)"
}
