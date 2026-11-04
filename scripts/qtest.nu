#!/usr/bin/env nu

# Vanllia acceptance orchestration: build, select board arguments, own helper
# lifetimes, invoke the pinned Kernel test runner and archive the evidence.
# Serial transport is acceptance.transport.feed; evidence verdicts belong to
# acceptance/verdicts.nu and the namespaced Python oracles. Neither is embedded.
#
# Only product has an available image deployment. No-scene health deliberately
# retains its missing-image sentinel. Other scenes fail before side effects;
# their retained descriptions/oracles are preparation for later provider intake.
#
# Runtime examples (require separate runtime authorization):
#   nu scripts/qtest.nu --package kernel --scene product
#   nu scripts/qtest.nu --scene product --feed-driver acceptance.drivers.shell.smoke
#   nu scripts/qtest.nu --scene product --feed-script /absolute/user/driver.py
# A driver module and an external script are mutually exclusive. Built-ins use
# normal package imports; only explicit external scripts use an extension loader.
# Host-only validation: python3 -B -m unittest discover -s tests -t .

use ./qemu-args.nu chosen-sbi
use ../acceptance/scenes.nu describe
use ../acceptance/verdicts.nu verify

# Serial helper: acceptance.transport.feed, launched from the explicit repository cwd.

# QEMU 命令行改写器（`--scene` 时落到 `<TRACE_OUT>/scene-qemu.sh`，经 `cargo qtest --qemu` 递）。
# **只摘掉 `cargo-qtest` 硬编码的 `-nographic`**——它会隐式把 serial0 绑到 stdio，把我们那一枚
# `-serial` 挤到 serial1（guest 只跟 UART0 说话）。其余参数原样透传。
# **（诊断：留全份控制台）** `cargo qtest` 在失败时只印 QEMU 输出**尾若干行**（超时那一档尤其
# 只剩尾巴），而"机器跑到哪一步停住"恰恰要看**前面**。`QEMU_TEE_LOG` 给了就顺路拷一份全份；
# exec 保持 runner 所持 PID 就是 QEMU；捕获进程不隔开超时终止责任。
const WRAPPER = '#!/usr/bin/env bash
set -o pipefail
cd "$QTEST_SOURCE_ROOT"
args=()
for a in "$@"; do
  [ "$a" = "-nographic" ] || args+=("$a")
done
if [ -n "$QEMU_TEE_LOG" ]; then
  exec qemu-system-riscv64 "${args[@]}" > >(
    if tee "$QEMU_TEE_LOG"; then
      printf 0 > "$QEMU_TEE_STATUS"
    else
      printf 1 > "$QEMU_TEE_STATUS"
    fi
  ) 2>&1
else
  exec qemu-system-riscv64 "${args[@]}"
fi'

def --wrapped main [--package: string, --scene: string = "", --profile: string, --feed: string, --feed-script: string, --feed-driver: string, --feed-after: int = 5, ...rest: string] {
  if $rest == ["--help"] {
    print "qtest.nu — run the kernel's embedded scene (health sentinel or product) under QEMU."
    print "  --package <name>   workspace to run (default kernel)"
    print "  --scene <name>     image scene (empty, or product)"
    print "  --profile <name>   debug or release (default release for scenes)"
    return
  }
  let scene_gate = if ($scene | is-empty) or ($scene == "product") { true } else { false }
  if not $scene_gate {
    error make {msg: "scene image not migrated to Vanllia; migrate deployment/image providers first"}
  }
  if not ($profile | is-empty) and ($profile not-in ["debug" "release"]) {
    error make {msg: $"profile 只接受 debug 或 release，得到 ($profile)"}
  }
  if not ($feed_script | is-empty) and not ($feed_driver | is-empty) {
    error make {msg: "choose --feed-driver or --feed-script"}
  }
  if not ($feed_driver | is-empty) and not ($feed_driver | str starts-with "acceptance.drivers.") {
    error make {msg: "built-in driver must belong to acceptance.drivers"}
  }
  if $feed_after < 0 {
    error make {msg: "--feed-after must be nonnegative"}
  }
  # Freeze the firmware before helpers, cargo or QEMU run.
  $env.SHUYRA_SBI = ((chosen-sbi) | into string)
  if ($env.QEMU_ICOUNT? | is-empty) { $env.QEMU_ICOUNT = "" }

  # **runner 守卫**：crates.io 上**两个包**都提供 `cargo-qtest` 这一枚 bin——
  # `cargo-qemu-test`（对每一例起一个 QEMU，本脚本按它写）与 `cargo-qtest`（一个"挑用例的
  # UI"包装器，**一个 qemu 字样都没有**）。装错的那一枚不认识 `--qemu-arg`，而它的报错长成
  # `error: unexpected argument '--qemu-arg' found`——看起来像本脚本的旗标写错了（实测误判过）。
  # 故先问一句 `--help`：那一格在 ⇒ 是对的那一枚。
  let ok_runner = (try {
    (^cargo qtest --help | complete | get stdout) | str contains "--qemu-arg"
  } catch { false })
  if not $ok_runner {
    error make {msg: "cargo-qtest 不是 cargo-qemu-test——装成了同名的另一个包（挑用例的 UI）。装对的那一枚：`cargo install cargo-qemu-test --version 0.3.2 --target x86_64-unknown-linux-gnu --locked --force`"}
  }

  let script_dir = $env.FILE_PWD
  let repo = ($script_dir | path dirname)
  $env.VANLLIA_DIAGNOSE = ($repo | path join "sqware-diagnose.jsonl")
  let trapdir = (($env.TRACE_OUT? | default ($repo | path join "trace")) | path expand)
  let selected_package = if ($package | is-empty) { "kernel" } else { $package }
  let kernel_workspace = ($repo | path join "components" "kernel")
  let run_cwd = if ($selected_package == "kernel") { $kernel_workspace } else { $repo }
  if not (($run_cwd | path join "Cargo.toml") | path exists) {
    error make {msg: $"qtest 包工作区不存在：($run_cwd)"}
  }
  let plan = (describe $scene)
  let retirement = $plan.retirement
  let hardware = $plan.hardware
  let readback = $plan.readback
  let soak = $plan.soak
  let dual = $plan.dual
  let dual_generations = $plan.dual_generations
  let soak_generations = $plan.soak_generations
  let confirmation = $plan.confirmation
  let guard = $plan.guard
  let guard_target = $plan.guard_target
  let daemon_rtc = $plan.daemon_rtc
  let daemon_target = $plan.daemon_target
  let daemon_stop = $plan.daemon_stop
  let daemon = $plan.daemon
  let evidence = $plan.evidence
  let retire_mode = $plan.retire_mode
  let feed_driver = if ($retirement or $hardware or $readback or $guard) and ($feed_script | is-empty) and ($feed_driver | is-empty) {
    let driver = if $hardware { "hardware" } else if $readback { "readback" } else if $guard { "guard" } else { $retire_mode }
    $"acceptance.drivers.device_recovery.($driver)"
  } else { $feed_driver | default "" }
  let feed_script = if ($feed_script | is-empty) { "" } else { $feed_script | path expand }
  let driven = not ($feed_script | is-empty) or not ($feed_driver | is-empty)
  let hold = if $retirement or $hardware or $readback or $guard {
    ($repo | path join $"target/ruin-(random uuid).nbd")
  } else {
    ""
  }
  $env.DMA_HOLD = $hold
  $env.DMA_CONTROL = if $hardware or $readback { ($repo | path join $"target/control-(random uuid).qtest") } else { "" }
  $env.DMA_HOLD_SECONDS = if $guard { "60" } else if $scene == "iommu-forced" { "30" } else { "10" }
  let summary = match $scene {
    "memory-transfer" => "scene-memory-transfer-phases.json"
    "memory-transfer-tlb" => "scene-memory-transfer-tlb-summary.json"
    "memory-transfer-fresh" => "scene-memory-transfer-fresh-summary.json"
    "memory-transfer-race" => "scene-memory-transfer-race-summary.json"
    _ => ""
  }
  let pending = if ($summary | is-empty) {
    ""
  } else {
    rm --force ($trapdir | path join $summary)
    $trapdir | path join $"($summary).pending-(random uuid)"
  }
  $env.SCENE_SUMMARY_PENDING = $pending

  mut cap = ""
  mut scene_args = []
  mut serial = ""
  mut wrapper = ""

  # 景：先造镜像，再把 `-initrd` 指过去（**绝对路径**——runner 的工作目录不是仓根），
  # 并把串口换到一条我们能喂输入的通道上（见头注）。
  # 档默认 release：`cargo image` 自己也是这个默认，而 `rig` 说 release 是
  # 它跑得动的前提（debug 下每轮都挂在 20 ms 那一缝上）。
  if not ($scene | is-empty) {
    let prof = if ($profile | is-empty) { "release" } else { $profile }
    # 产品那四景各有自己的部署（入口镜像按景名认）：编排都在 Harness 那两台单元里，
    # 故场景名→镜像名只做一次映射，不给生产镜像塞景码。
    let image_scene = match $scene {
      "iommu-product-start-before" => "iommu-product-before"
      "iommu-product-start-after" => "iommu-product-after"
      _ => $scene
    }
    let image = (do {
      cd $repo
      ^cargo image build $image_scene $prof | complete
    })
    if $image.exit_code != 0 {
      print -n $image.stdout
      print -n $image.stderr
      exit $image.exit_code
    }
    let at = (
      $image.stdout
      | lines
      | where { |l| ($l =~ '^ok\s+') }
      | last
      | str replace --regex '^ok\s+' ''
      | str trim
      | path expand
    )
    if not ($at | path exists) or (($at | path type) != "file") {
      error make {msg: $"造不出镜像：cargo image ($scene) ($prof) 说落在 ($at)"}
    }
    print $"景 ($scene)（档 ($prof)）· 镜像 ($at)"
    $scene_args = ["--qemu-arg=-initrd" $"--qemu-arg=($at)"]

    mkdir $trapdir
    if $hardware or $readback {
      if (which dtc | is-empty) {
        error make {msg: "硬件故障场景需要 dtc 读取 QEMU 生成的设备树"}
      }
      $env.DMA_DTB = ($trapdir | path join "board.dtb")
      let dump_args = (^nu ($script_dir | path join "qemu-args.nu") --board-only --iommu | lines)
      let dump = (^qemu-system-riscv64 ...$dump_args -machine $"dumpdtb=($env.DMA_DTB)" | complete)
      if $dump.exit_code != 0 {
        error make {msg: $"设备树生成失败：($dump.stderr)"}
      }
    }
    if ($env.QEMU_TEE_LOG? | is-empty) {
      $env.QEMU_TEE_LOG = ($trapdir | path join $"scene-($scene)-console.log")
    } else {
      $env.QEMU_TEE_LOG = ($env.QEMU_TEE_LOG | path expand)
    }
    $env.QEMU_TEE_STATUS = ($trapdir | path join $"scene-($scene)-capture.status")
    "P" | save --force $env.QEMU_TEE_STATUS
    $cap = ($trapdir | path join $"scene-($scene)-feed.log")
    let pid_file = ($trapdir | path join "scene-feed.pid")
    let port_file = ($trapdir | path join "scene-feed.port")
    let err_file = ($trapdir | path join "scene-feed.err")
    let text = if ($feed | is-empty) { "\u{4}" } else { $feed }
    rm -f $port_file
    if $driven { rm -f $"($cap).result" }
    # 只使用本轮导出；正式成功标记仅由本进程发布。
    if not ($summary | is-empty) or $evidence {
      rm --force ($repo | path join "sqware-diagnose.jsonl")
      rm --force $cap
      rm --force $env.QEMU_TEE_LOG
      rm --force ($trapdir | path join $"scene-($scene)-diagnose.jsonl")
    }
    $env.FEED_DRIVER = $feed_driver
    $env.FEED_PID = $pid_file
    $env.FEED_PORTFILE = $port_file
    $env.FEED_CAP = $cap
    $env.FEED_TEXT = $text
    $env.FEED_AFTER = ($feed_after | into string)
    $env.FEED_SCRIPT = (if ($feed_script | is-empty) { "" } else { $feed_script | path expand })
    $env.FEED_ERR = $err_file
    # 上一轮万一留了帮手，先收掉（它自己会一直 listen）。
    if ($pid_file | path exists) {
      ^bash -c 'p=$(cat "$FEED_PID"); kill "$p" 2>/dev/null; true'
    }
    $env.QTEST_SOURCE_ROOT = ($repo | into string)
    $env.QTEST_RUN_CWD = ($repo | into string)
    ^bash -c 'cd "$QTEST_RUN_CWD" || exit
extra=()
if [ -n "$FEED_DRIVER" ]; then extra=(--driver "$FEED_DRIVER"); fi
if [ -n "$FEED_SCRIPT" ]; then extra=(--script "$FEED_SCRIPT"); fi
nohup python3 -m acceptance.transport.feed "$FEED_PORTFILE" "$FEED_CAP" "$FEED_TEXT" "$FEED_AFTER" "${extra[@]}" >"$FEED_ERR" 2>&1 &
echo $! > "$FEED_PID"'
    # 端口由帮手 bind(0) 后报出来（有界等；它没起来就当场说清楚，不要等到十例都超时）。
    let got = (^bash -c 'for i in $(seq 1 50); do if [ -s "$FEED_PORTFILE" ]; then cat "$FEED_PORTFILE"; exit 0; fi; sleep 0.1; done; exit 1' | complete)
    if ($got.exit_code != 0) {
      print $"喂入帮手没起来（($err_file)）："
      ^cat $err_file
      ^bash -c 'p=$(cat "$FEED_PID"); kill "$p" 2>/dev/null; true'
      error make {msg: "喂入帮手没起来——python3 在不在 PATH 里？见上面那份 stderr"}
    }
    $serial = $"tcp:127.0.0.1:($got.stdout | str trim)"

    # 改写器：`cargo qtest` 要一个**能直接执行**的路径。
    $wrapper = ($trapdir | path join "scene-qemu.sh")
    $WRAPPER | save --force $wrapper
    $env.QEMU_WRAPPER = $wrapper
    ^bash -c 'chmod +x "$QEMU_WRAPPER"'
    let label = if not ($feed_driver | is-empty) { $feed_driver } else if not ($feed_script | is-empty) { $"交互脚本 ($feed_script)" } else if ($feed | is-empty) { "EOF" } else { $feed }
    print $"喂入 ($label)（重喂到机器退）· 串口 ($serial) · 捕获 ($cap)"
  }

  # 参数表**唯一出处**：串口那一格也归它（`--serial`），本文件不写板子字面量。
  # `--iommu` 也只经它那一口传：只有要那件硬件的景（`iommu-*`）才开。
  let serial_argv = if ($serial | is-empty) { [] } else { ["--serial" $serial] }
  let iommu_argv = if ($scene | is-empty) {
    []
  } else if ($scene | str starts-with "iommu") {
    ["--iommu"]
  } else {
    []
  }
  # 端点那一景的那张盘：**可再生**（每次按同一花纹重建，落在 `<TRACE_OUT>/` 里，
  # 不进库），且由参数表那边只读挂上。景那一侧按同一花纹核读回的数据，故盘是承重的。
  let endpoint_argv = if ($scene in ["iommu-dma" "iommu-product" "iommu-product-fault" "iommu-product-start-before" "iommu-product-start-after" "iommu-ruin" "iommu-forced" "iommu-soak" "iommu-soak-long" "iommu-fail-invalidate" "iommu-fail-reset" "iommu-fail-release" "iommu-fail-reference" "iommu-fail-fence"]) or $readback or $guard or $dual {
    let disk = ($trapdir | path join "iommu-dma.img")
    ^python3 -c 'import sys, pathlib; pathlib.Path(sys.argv[1]).write_bytes(bytes(range(256)) * 4096)' $disk
    if not ($hold | is-empty) {
      $env.DMA_HOLD_DISK = $disk
      $env.DMA_HOLD_PID = ($trapdir | path join "block.pid")
      $env.DMA_HOLD_ERR = ($trapdir | path join "block.err")
      ^bash -c 'cd "$QTEST_SOURCE_ROOT" || exit
nohup python3 -m acceptance.fixtures.block_device "$DMA_HOLD" "$DMA_HOLD_DISK" >"$DMA_HOLD_ERR" 2>&1 & echo $! > "$DMA_HOLD_PID"'
      let ready = (^python3 -c 'import pathlib, sys, time; p=pathlib.Path(sys.argv[1]+".json"); end=time.monotonic()+5
while not p.exists() and time.monotonic()<end: time.sleep(.01)
sys.exit(0 if p.exists() else 1)' $hold | complete)
      if $ready.exit_code != 0 {
        ^cat $env.DMA_HOLD_ERR
        ^bash -c 'kill "$(cat "$DMA_HOLD_PID")" "$(cat "$FEED_PID")" 2>/dev/null; true'
        error make {msg: "在途块设备夹具未启动"}
      }
    }
    if $dual {
      let second = ($trapdir | path join "iommu-dma-second.img")
      ^python3 -c 'import sys, pathlib; pathlib.Path(sys.argv[1]).write_bytes(bytes(n ^ 0xa5 for n in range(256)) * 4096)' $second
      ["--endpoint" $disk "--endpoints" ([$second] | to json --raw)]
    } else {
      ["--endpoint" $disk]
    }
  } else {
    []
  }
  let hold_argv = if ($hold | is-empty) { [] } else { ["--endpoint-hold" $hold] }
  let control_argv = if $hardware or $readback { ["--qtest-socket" $env.DMA_CONTROL] } else { [] }
  let board = (^nu ($script_dir | path join "qemu-args.nu") --board-only ...$iommu_argv ...$serial_argv ...$endpoint_argv ...$hold_argv ...$control_argv
      | lines | where { |l| ($l | str trim) != "" })
  let qemu_args = ($board | each { |a| $"--qemu-arg=($a)" })
  # The Kernel SHA is read only when Kernel is the selected package. A
  # non-kernel package must work without an initialized components/kernel.
  let kernel_sha = if ($selected_package == "kernel") {
    (^git -C $kernel_workspace rev-parse HEAD | str trim)
  } else {
    "not-selected"
  }
  let pkg = ["--package" $selected_package]
  print $"qtest: cwd=($run_cwd) package=($selected_package) workspace=($kernel_workspace) kernel_sha=($kernel_sha)"
  let qemu_bin = if ($wrapper | is-empty) { [] } else { ["--qemu" $wrapper] }
  # 整机用例跑 **release**：同一个 `accept` 景，release 产品路 6/6 稳（14 笔结局），debug 产品路
  # 结局笔数 5 / 11 / 6 / 11 乱跳、偶发 panic ⇒ debug 档下这条世界本来就不可靠。健康面那九例
  # 是 `#[cfg(debug_assertions)]`（它们的身子在 `kernel::health` 里），故 release 档的测试
  # 目标只剩整机那一例——正好，也快。
  let test_profile = if ($scene | is-empty) { [] } else { ["-r"] }

  # **归档（O10）**：`cargo qtest` 的输出**就是**这一轮的现场——机侧控制台走 semihosting
  # 落在它的 stdout 里（见头注），而测试路没有结构化导出（内核那个 `semihosting` feature
  # 不在测试构建里）。故与 `runner.nu` 同款纪律：**失败留档、正常那轮丢掉**。
  # `o+e>| tee { save }` 既保终端流、又保退出码（`try` 那两条地雷见 `runner.nu` 头注）。
  mkdir $trapdir
  let qtlog = ($trapdir | path join (if ($scene | is-empty) { "qtest.log" } else { $"scene-($scene)-qtest.log" }))
  # 命令与重定向**必须同一行**（分行 ⇒ `nu::parser::unexpected_redirection`）。故先把 argv
  # 拼好，再一行发出去——退出码仍由 `LAST_EXIT_CODE` 拿（`try` 那两条地雷见 `runner.nu` 头注）。
  # `--wrapped` 已让未知 runner flags 进入 `rest`；只剥一个 CLI 分隔符 `--`，
  # 其余 flag 的值、顺序与数量原样递下去。
  let rest = if (($rest | length) > 0) {
    if (($rest | first) == "--") { $rest | skip 1 } else { $rest }
  } else {
    $rest
  }
  let qtest_argv = ["qtest" "--target" "riscv64gc-unknown-none-elf" ...$test_profile ...$qemu_bin ...$qemu_args ...$scene_args ...$pkg ...$rest]
  # **工作区那三条 rustflags 要由这一侧递一遍**：`cargo-qtest` 把 `RUSTFLAGS` 的值
  # 当作**种子**，再拼上它自己那两枚（对我们这一景是 `-Tembedded-test.x`），最后经
  # `cargo --config <临时 TOML>` 投下去——而那一份 config 的 `[target.'cfg(target_arch="riscv64")']`
  # 表**优先于**根 `.cargo/config.toml` 的 `[build] rustflags` ⇒ 不递的话那三条会静默丢。
  # **真相仍只有一处**（`.cargo/config.toml`）；这里重复的是"调用方给 runner 递种子"这一手，
  # 不是第二处定义。已显式设过 `RUSTFLAGS` 就不动它。
  if ($env.RUSTFLAGS? | is-empty) {
    $env.RUSTFLAGS = "-Crelocation-model=static -Cforce-frame-pointers=yes -Ccode-model=medium"
  }
  # `try` 的块作用域会丢掉外部命令的 LAST_EXIT_CODE；直接保留完整执行结果。
  # Only the actual cargo-qtest call runs from the Kernel provider workspace.
  # `--manifest-path` is unsupported by the installed runner (raw 2), so use a
  # scoped cwd closure and restore the Source cwd immediately afterwards.
  let here = ($env.PWD)
  let result = (do {
    cd $run_cwd
    ^cargo ...$qtest_argv o+e>| tee { save --force $qtlog } | complete
  })
  cd $here
  print -n $result.stdout
  let raw_code = $result.exit_code
  mut code = $raw_code
  if not ($scene | is-empty) {
    let capture_status = (try { open --raw $env.QEMU_TEE_STATUS | str trim } catch { "missing" })
    if $capture_status != "0" {
      print $"控制台捕获未成功完成：($capture_status)"
      # A guard halt is killed by the runner timeout; keep the raw 101 so the
      # independent verifier can judge the captured non-returning evidence.
      if not ($guard and ($code == 101)) {
        $code = 1
      }
    }
  }
  if $driven {
    let result = $"($cap).result"
    $env.FEED_RESULT = $result
    ^bash -c 'for i in $(seq 1 50); do if [ -s "$FEED_RESULT" ]; then exit 0; fi; sleep 0.02; done; true'
    if not ($result | path exists) {
      print "交互驱动未完成"
      $code = 1
    } else if (open --raw $result | str trim) != "ok" {
      print (open --raw $result)
      $code = 1
    }
  }

  $code = (try {
    verify {
      plan: $plan, raw_code: $raw_code, code: $code,
      repo: $repo, trapdir: $trapdir, cap: $cap, qtlog: $qtlog,
      summary: $summary, pending: $pending, hold: $hold
    }
  } catch {|failure|
    print -e $failure.msg
    1
  })

  if ($qtlog | path exists) {
    if $code == 0 and not $daemon {
      rm --force $qtlog
    } else if $code != 0 {
      print $"现场（cargo qtest 全份）-> ($qtlog)"
    }
  }

  if not ($scene | is-empty) {
    ^bash -c 'p=$(cat "$FEED_PID"); kill "$p" 2>/dev/null; true'
    if not ($hold | is-empty) {
      ^bash -c 'p=$(cat "$DMA_HOLD_PID"); kill "$p" 2>/dev/null; true'
    }
    # 串口是一根"只喂不读"的输入线；机侧控制台仍走 semihosting（见头注）。这一手是为
    # 串口路由万一被改动时留的后手——读不到东西是正常的。
    if $code != 0 and ($cap | path exists) {
      print $"（喂入侧捕获：($cap)，下面是尾 40 行）"
      ^tail -40 $cap
    }
  }
  exit $code
}
