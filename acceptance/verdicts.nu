# Evidence verdict owner. No build, feed process, or QEMU startup belongs here.
def --wrapped oracle [root: path, module: string, ...argv: string] {
  do {
    cd $root
    ^python3 -m $module ...$argv | complete
  }
}

export def verify [context: record] {
  let plan = $context.plan
  let scene = $plan.name
  let raw_code = $context.raw_code
  mut code = $context.code
  let repo = $context.repo
  let trapdir = $context.trapdir
  let cap = $context.cap
  let qtlog = $context.qtlog
  let summary = $context.summary
  let pending = $context.pending
  let hold = $context.hold
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

  let completion = match $scene {
    "accept" => "accept: done"
    "product" => "system: done"
    "daemon-rtc" => "daemon-rtc: done"
    "daemon-target" => "daemon-target: done"
    "daemon-stop" => "daemon-stop: done"
    "system-fault" => "system-fault: acceptance passed"
    _ => ""
  }
  if $code == 0 and not ($completion | is-empty) {
    let console = ($env.QEMU_TEE_LOG? | default "")
    let lines = if ($console | path exists) { open --raw $console | lines } else { [] }
    let entries = ($lines | parse -r '^boot: entry task=(?<task>[1-9][0-9]*)$')
    if ($entries | length) != 1 {
      print $"场景未完成：缺少唯一的实际入口任务身份 ($completion)"
      $code = 1
    } else {
      let task = ($entries | first | get task)
      let proof = $"^exit tid=($task) reason=0x0 note: ($completion)$"
      if ($lines | where {|line| $line =~ $proof } | length) != 1 {
        print $"场景未完成：缺少实际入口任务 ($task) 的唯一成功退出证明 ($completion)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene == "accept" {
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    for probe in ["probe-terminal" "probe-denied" "probe-owner" "probe-rule" "probe-rule-other" "probe-watch" "probe-watch-after" "probe-watch-gone" "probe-lease" "probe-bound" "probe-rack" "probe-rack-mount" "probe-rack-guest" "probe-coalition" "probe-control" "probe-operator-gate" "probe-operator-land" "subject" "member" "guest"] {
      let proof = $"^exit tid=[0-9]+ reason=0x0 note: ($probe):"
      if ($console | where {|line| $line =~ $proof } | is-empty) {
        print $"验收探针未成功完成：($probe)"
        $code = 1
      }
    }
    let retained = "^probe-lease: declared retention acquired, checked and released$"
    if ($console | where {|line| $line =~ $retained } | is-empty) {
      print "验收探针未验证声明供给的保留授权"
      $code = 1
    }
  }

  if $code == 0 and $scene == "memory-transfer" {
    # 两台都报各自那一句话：引导那一台报收尾（本景的成功证明），
    # 控制单元报自己也正常收了工。两句话都是本景独有的串，不按 tid 认——
    # 打开 `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS` 时，引导期那些自检会先铸几台任务，
    # 入口那一台的 tid 就不是 1 了（实测那一轮是 10）。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    for proof in ["memory-transfer: acceptance passed" "memory-transfer: run passed"] {
      let at = $"^exit tid=[0-9]+ reason=0x0 note: ($proof)"
      if ($console | where {|line| $line =~ $at } | is-empty) {
        print $"本景未成功完成：缺少 ($proof)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene == "memory-transfer-tlb" {
    # 两句都是本景独有的串：引导那一台报收尾（本景的成功证明），
    # 读者那一台报自己也真的出过那段热窗口。后者走 semihosting，没有 `exit tid=` 前缀。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    for proof in [
      "^exit tid=[0-9]+ reason=0x0 note: memory-transfer-tlb: acceptance passed"
      "^memory-transfer-tlb: reader left"
    ] {
      if ($console | where {|line| $line =~ $proof } | is-empty) {
        print $"本景未成功完成：缺少 ($proof)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene == "memory-transfer-fresh" {
    # 引导那一台报收尾：两阶段的读者都已被它 join 到手才走到这一行，故这一句就是
    # 本景的成功证明（逐格判据在宿主驱动那一侧）。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    let proof = "^exit tid=[0-9]+ reason=0x0 note: memory-transfer-fresh: acceptance passed"
    if ($console | where {|line| $line =~ $proof } | is-empty) {
      print $"本景未成功完成：缺少 ($proof)"
      $code = 1
    }
  }

  if $code == 0 and $scene == "memory-transfer-race" {
    # 引导那一台报收尾：对端已被它 join 到手才走到这一行；逐格判据（那一次真冷缺页、
    # 那一次真放行、两页读数一致、那条外空间关系真握着一条翻译）在宿主驱动那一侧。
    # 这里另外核**机器级净回收**：两台都正常退场、整机真的收场、没有留下的等待者——
    # 那几行是内核打到 semihosting 控制台上的，只有本脚本看得到。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    let want = [
      "^exit tid=[0-9]+ reason=0x0 note: memory-transfer-race: peer passed$"
      "^exit tid=[0-9]+ reason=0x0 note: memory-transfer-race: acceptance passed$"
      "^task: all tasks exited, system halted$"
      "^doom: held=0 starved=0 blocked=0 nudged=0$"
    ]
    for $pattern in $want {
      if ($console | where {|line| $line =~ $pattern } | is-empty) {
        print $"本景未成功完成：缺少 ($pattern)"
        $code = 1
      }
    }
  }

  if $code == 0 and not ($scene | is-empty) {
    let frames = (open --raw $env.QEMU_TEE_LOG | lines
      | parse -r '^memory: frame_allocations before=(?<before>[0-9]+) after=(?<after>[0-9]+)$')
    if ($frames | length) != 1 or ($frames | any {|row| $row.before != $row.after }) {
      print "本景未成功完成：物理帧分配未回到起场基线"
      $code = 1
    }
  }

  if $evidence {
    let exported = ($repo | path join "sqware-diagnose.jsonl")
    let archived = ($trapdir | path join $"scene-($scene)-diagnose.jsonl")
    if ($exported | path exists) { cp --force $exported $archived }
    if $guard {
      let nbd_events = ($trapdir | path join $"scene-($scene)-block-events")
      let nbd_state = ($trapdir | path join $"scene-($scene)-block-state.json")
      if (not ($hold | is-empty)) and (($"($hold).events") | path exists) {
        cp --force $"($hold).events" $nbd_events
        cp --force $"($hold).json" $nbd_state
      }
      let harts = ($env.QEMU_SMP? | default "4")
      let checked = (oracle $repo "acceptance.oracles.device_recovery.guard"
        $env.QEMU_TEE_LOG $archived $cap $guard_target $harts ($code | into string) $qtlog --hold $hold)
      print -n $checked.stdout
      print -n $checked.stderr
      if $checked.exit_code != 0 { $code = 1 }
    } else if $daemon {
      let checker = if $daemon_rtc {
        "acceptance.oracles.daemon.rtc"
      } else if $daemon_target {
        "acceptance.oracles.daemon.target"
      } else {
        "acceptance.oracles.daemon.stop"
      }
      let checked = (oracle $repo $checker
        $env.QEMU_TEE_LOG $archived $qtlog ($raw_code | into string))
      print -n $checked.stdout
      print -n $checked.stderr
      if $checked.exit_code != 0 { $code = 1 }
    } else {
      let checker = if $retirement {
        {script: "acceptance.oracles.device_recovery.ruin", args: [$cap $retire_mode]}
      } else if $confirmation {
        {script: "acceptance.oracles.device_recovery.confirmation", args: [($scene | str replace "iommu-fail-" "")]}
      } else if $hardware {
        {script: "acceptance.oracles.device_recovery.hardware", args: [$cap]}
      } else if $readback {
        {script: "acceptance.oracles.device_recovery.readback", args: [$cap]}
      } else if $dual {
        {script: "acceptance.oracles.device_recovery.dual", args: [($dual_generations | into string)]}
      } else {
        {script: "acceptance.oracles.device_recovery.soak", args: [($soak_generations | into string)]}
      }
      if $code == 0 {
        let checked = (oracle $repo $checker.script
          $env.QEMU_TEE_LOG $archived ...$checker.args)
        print -n $checked.stdout
        print -n $checked.stderr
        if $checked.exit_code != 0 { $code = 1 }
      }
    }
  }

  if $code == 0 and $scene == "iommu-probe" {
    # 引导那一台报收尾：拆除与隔离判据都在它自己那一侧逐条核过（身子见 `probe/iommu.rs`）。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    let proof = "^exit tid=[0-9]+ reason=0x0 note: iommu-probe: acceptance passed"
    if ($console | where {|line| $line =~ $proof } | is-empty) {
      print $"本景未成功完成：缺少 ($proof)"
      $code = 1
    }
    for coordinate in [
      "64 iova=0x208000000"
      "511 iova=0x23fe00000"
      "512 iova=0x240000000"
      "126975 iova=0x3fffe00000"
    ] {
      let window = $"^iommu-probe: window boundary=($coordinate) response=0x[0-9a-f]+ previous=0x[0-9a-f]+ stale_response=0x[0-9a-f]+ ddtp_off=0x0 cqcsr=0x0 fqcsr=0x0$"
      if ($console | where {|line| $line =~ $window } | length) != 1 {
        print $"本景未成功完成：缺少唯一的窗口翻译与旧窗拒绝读数 ($coordinate)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene in ["iommu-product" "iommu-product-fault"] {
    # 产品那一侧：System 只装配（UnitFile/Deployment/Assembly/Control/Supply），
    # PCI／IOMMU／设备恢复各守本硬件职责（各自独立 Task/Space），Hub 只做发现、绑定、
    # 停止新请求与恢复协调；场景编排（固定扇区／哨兵／杀服务）在 Harness 那两台单元里。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    mut want = [
      # 三个硬件域各自独立 Task/Space：各报自己的号与它自己映射的那一段（本域 VA）。
      "^iommu: space task=[0-9]+ iommu=0x[0-9a-f]+ retain=[0-9]+ ddtp=0x0 \\(off\\)$"
      "^pci: space task=[0-9]+ ecam=0x[0-9a-f]+ functions=[0-9]+$"
      "^pci: ready functions=[0-9]+$"
      "^virtio-blk: ready hub=[0-9]+ pci=[0-9]+ requester=[0-9]+ key=[0-9]+$"
      # 执行实例由控制器用受限构造建出（owner = Hub），再由 Hub 放行；
      # 停止与核销也归 Hub（实例面按真实 owner 核）。
      "^ctl: constructed device-work tid=[0-9]+ owner=hub$"
      "^hub: execution instance launched tid=[0-9]+$"
      "^ctl: binding 1 auxiliary=[0-9]+ alive=true$"
      "^ctl: binding 1 auxiliary=[0-9]+ alive=false$"
      # 落位与开启分开：落位时 MASTER 是关的（真回读），IOMMU 发布、恢复映射与子授权都
      # 就绪之后才开。
      "bus-master off=0x0$"
      "^pci: endpoint bus-master on=0x4$"
      # Hub 放行业务：发布之后才 Active（执行侧从它那一条孔拿到坐标）。
      "^iommu: binding 1 published requester=[0-9]+ held=6"
      "^virtio-blk: binding 1 active iova=0x[0-9a-f]+ execution=[0-9]+$"
      # 固定工作负载：真读一个扇区、花纹对得上、状态是 OK（判据在控制器那一侧）。
      "^ctl: served used=1 len=513 written_ok=true sector_ok=true status=0$"
      # 收场：先整域停执行方 CPU 控制，再按真实停链协调。
      "^hub: binding 1 execution domain stopped \\(tid=[0-9]+\\)$"
      "^iommu: binding 1 invalidated frames held=6 \\(not released\\)$"
      "^pci: endpoint bus-master off=0x0$"
      "^virtio-blk: binding 1 stopped reset=1 status=0 queue_enable=0$"
      "^iommu: binding 1 queues stopped cqcsr=0x0 fqcsr=0x0 pqcsr=0x0 queue_enable=0 faults=0 last_did=0$"
      # 每一个字段都钉死：把停链换成空实现，这几格就回不到 0；帧在停链期间一直持着。
      "^iommu: binding 1 released ddtp=0x0 cqcsr=0x0 fqcsr=0x0 pqcsr=0x0 queue_enable=0 frames=0 stopped=true$"
      "^virtio-blk: binding 1 detached$"
      "^hub: binding 1 closed and released device [0-9]+$"
      # 整机也真的收场（与普通 product 同一格）：任务都退了、没有残余持有。
      "^exit tid=1 reason=0x0 note: system: done$"
      "^task: all tasks exited, system halted$"
      "^memory: frame_allocations before=[0-9]+ after=[0-9]+$"
    ]
    if $scene == "iommu-product" {
      $want = ($want | append [
        "^work: requesting Close::Normal$"
        "^ctl: binding 1 closed and reclaimed$"
      ])
    } else {
      $want = ($want | append [
        "^ctl: constructed device-work tid=[0-9]+ owner=hub$"
        "^hub: execution instance launched tid=[0-9]+$"
        "^ctl: alpha exit requested \\(hub recovers owner death\\)$"
        "^ctl: rebind probe claimed serial=2 \\(device free again\\)$"
        "^virtio-blk: binding 2 active iova=0x[0-9a-f]+ execution=[0-9]+$"
        "^work: rang the stale iova doorbell$"
        "^ctl: stale iova submitted sentinel_held=true status_held=true$"
        "^iommu: binding 2 queues stopped cqcsr=0x0 fqcsr=0x0 pqcsr=0x0 queue_enable=0 faults=[1-9][0-9]* last_did=[0-9]+$"
        "^iommu: binding 2 released ddtp=0x0 cqcsr=0x0 fqcsr=0x0 pqcsr=0x0 queue_enable=0 frames=0 stopped=true$"
        "^hub: binding 2 closed and released device [0-9]+$"
        "^ctl: binding 2 auxiliary=[0-9]+ alive=true$"
        "^ctl: binding 2 auxiliary=[0-9]+ alive=false$"
      ])
    }
    for $pattern in $want {
      if ($console | where {|line| $line =~ $pattern } | is-empty) {
        print $"本景未成功完成：缺少 ($pattern)"
        $code = 1
      }
    }
    let pci = ($console | parse -r '^pci: space task=(?<task>[0-9]+) ecam=0x[0-9a-f]+ functions=(?<functions>[0-9]+)$')
    let pci_ready = ($console | parse -r '^pci: ready functions=(?<functions>[0-9]+)$')
    let drivers = ($console | parse -r '^virtio-blk: ready hub=(?<hub>[0-9]+) pci=(?<pci>[0-9]+) requester=(?<requester>[0-9]+) key=(?<key>[0-9]+)$')
    let hubs = ($console | parse -r '^system: minted hub tid=(?<task>[0-9]+)$')
    mut requester = ""
    mut key = ""
    if ($pci | length) != 1 or ($pci_ready | length) != 1 or ($drivers | length) != 1 or ($hubs | length) != 1 {
      print "本景未成功完成：PCI/Driver 身份不唯一"
      $code = 1
    } else {
      let pci_task = ($pci | first | get task)
      $requester = ($drivers | first | get requester)
      $key = ($drivers | first | get key)
      if ($drivers | first | get pci) != $pci_task or ($drivers | first | get hub) != ($hubs | first | get task) or ($pci_ready | first | get functions) != ($pci | first | get functions) {
        print "本景未成功完成：PCI/Driver 身份未对上"
        $code = 1
      }
      mut linked = [
        $"^iommu: binding 1 published requester=($requester) held=6"
        $"^hub: binding 1 closed and released device ($key)$"
      ]
      if $scene == "iommu-product-fault" {
        $linked = ($linked | append [
          $"^hub: binding 2 closed and released device ($key)$"
          $"^iommu: binding 2 queues stopped .* faults=[1-9][0-9]* last_did=($requester)$"
        ])
      }
      for $pattern in $linked {
        if ($console | where {|line| $line =~ $pattern } | is-empty) {
          print $"本景未成功完成：设备身份未对上 ($pattern)"
          $code = 1
        }
      }
    }
    # 停链次序是合同：撤翻译 → 清 MASTER → 设备 reset/drain → 关队列 → 释放；
    # 帧在确认之前一直持着；释放之后才放下恢复映射，才允许重绑。
    let ordered = [
      ["^hub: binding 1 execution domain stopped" "^iommu: binding 1 invalidated"]
      ["^iommu: binding 1 invalidated" "^pci: endpoint bus-master off=0x0$"]
      ["^pci: endpoint bus-master off=0x0$" "^virtio-blk: binding 1 stopped"]
      ["^virtio-blk: binding 1 stopped" "^iommu: binding 1 queues stopped"]
      ["^iommu: binding 1 queues stopped" "^iommu: binding 1 released"]
      ["^iommu: binding 1 invalidated" "^iommu: binding 1 released"]
      ["^iommu: binding 1 released" "^virtio-blk: binding 1 detached"]
      ["^virtio-blk: binding 1 detached" $"^hub: binding 1 closed and released device ($key)$"]
    ]
    for $pair in $ordered {
      let before = ($console | enumerate | where {|row| $row.item =~ $pair.0 } | get index | first)
      let after = ($console | enumerate | where {|row| $row.item =~ $pair.1 } | get index | first)
      if ($before | is-empty) or ($after | is-empty) or ($before >= $after) {
        print $"本景未按合同次序收口：($pair.0) 必须早于 ($pair.1)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene in ["iommu-product-start-before" "iommu-product-start-after"] {
    # 起手受控失败那两景：执行方在一个**明确的点**上停（发布之前／发布之后），
    # 由 Hub 按同一道门与同一条停链结清。要看到的是：没留活僵尸、本次责任真被结清、
    # 且两档之间设备那一侧的现场截然不同——发布之前失败 = 设备从来没被接上；
    # 发布之后失败 = 平台真把设备停了才释放。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    let pci = ($console | parse -r '^pci: space task=(?<task>[0-9]+) ecam=0x[0-9a-f]+ functions=(?<functions>[0-9]+)$')
    let pci_ready = ($console | parse -r '^pci: ready functions=(?<functions>[0-9]+)$')
    let drivers = ($console | parse -r '^virtio-blk: ready hub=(?<hub>[0-9]+) pci=(?<pci>[0-9]+) requester=(?<requester>[0-9]+) key=(?<key>[0-9]+)$')
    let hubs = ($console | parse -r '^system: minted hub tid=(?<task>[0-9]+)$')
    mut key = ""
    if ($pci | length) != 1 or ($pci_ready | length) != 1 or ($drivers | length) != 1 or ($hubs | length) != 1 {
      print "本景未成功完成：PCI/Driver 身份不唯一"
      $code = 1
    } else {
      let pci_task = ($pci | first | get task)
      $key = ($drivers | first | get key)
      if ($drivers | first | get pci) != $pci_task or ($drivers | first | get hub) != ($hubs | first | get task) or ($pci_ready | first | get functions) != ($pci | first | get functions) {
        print "本景未成功完成：PCI/Driver 身份未对上"
        $code = 1
      }
    }
    mut want = [
      "^pci: space task=[0-9]+ ecam=0x[0-9a-f]+ functions=[0-9]+$"
      "^pci: ready functions=[0-9]+$"
      "^virtio-blk: ready hub=[0-9]+ pci=[0-9]+ requester=[0-9]+ key=[0-9]+$"
      "^work: claimed serial=1$"
      "^ctl: constructed device-work-(before|after) tid=[0-9]+ owner=hub$"
      "^ctl: binding 1 auxiliary=[0-9]+ alive=true$"
      "^ctl: binding 1 auxiliary=[0-9]+ alive=false$"
      "^hub: execution instance launched tid=[0-9]+$"
      "^hub: binding 1 execution domain stopped \\(tid=[0-9]+\\)$"
      "^iommu: binding 1 released ddtp=0x0 cqcsr=0x0 fqcsr=0x0 pqcsr=0x0 queue_enable=0 frames=0 stopped=true$"
      "^virtio-blk: binding 1 detached$"
      $"^hub: binding 1 closed and released device ($key)$"
      "^exit tid=1 reason=0x0 note: system: done$"
      "^task: all tasks exited, system halted$"
      "^memory: frame_allocations before=[0-9]+ after=[0-9]+$"
    ]
    if $scene == "iommu-product-start-before" {
      $want = ($want | append [
        # 发布之前中止：从来没有过“已发布”那一行，执行方也没交过窗。
        "^work: requesting Close::Fault before publish$"
        "^ctl: binding 1 settled after abort$"
      ])
      let forged = (
        $console
        | where {|line| $line =~ "^iommu: binding 1 published" }
        | length
      )
      if $forged != 0 {
        print "本景未按预期收口：发布之前中止不该出现已发布那一行"
        $code = 1
      }
    } else {
      $want = ($want | append [
        # 发布之后中止：发布真发生过（撤翻译与整条停链才跑得有意义）。
        "^work: requesting Close::Fault after publish$"
        "^iommu: binding 1 published"
        "^iommu: binding 1 invalidated frames held=6 \\(not released\\)$"
        "^ctl: binding 1 settled after abort$"
      ])
    }
    for $pattern in $want {
      if ($console | where {|line| $line =~ $pattern } | is-empty) {
        print $"本景未成功完成：缺少 ($pattern)"
        $code = 1
      }
    }
  }

  if $code == 0 and $scene == "iommu-dma" {
    # 同理：端点那一景的正证与负证都在那一台自己那一侧逐条核过（身子见 `probe/iommu/dma.rs`）。
    let console = (open --raw $env.QEMU_TEE_LOG | lines)
    for proof in [
      "^pci-probe: reset-zero requester=[0-9]+ bar=[0-5] size=0x[0-9a-f]+ restored=0 master=0$"
      "^pci-probe: upper-half requester=[0-9]+ bar=[1-5] rejected=true restored=true$"
      "^exit tid=[0-9]+ reason=0x0 note: iommu-dma: acceptance passed"
    ] {
      if ($console | where {|line| $line =~ $proof } | is-empty) {
        print $"本景未成功完成：缺少 ($proof)"
        $code = 1
      }
    }
  }

  if $code == 0 and not ($summary | is-empty) {
    if ($pending | path exists) {
      mv --force $pending ($trapdir | path join $summary)
    } else {
      print "交互驱动未提交本轮判定文件"
      $code = 1
    }
  }

  $code
}
