# Scenario facts shared by the transport and evidence verifier.
export def describe [scene: string] {
  let retirement = $scene in ["iommu-ruin" "iommu-forced"]
  let hardware = $scene == "iommu-fail-fence"
  let readback = $scene == "iommu-fail-readback"
  let soak = $scene in ["iommu-soak" "iommu-soak-long"]
  let dual = $scene in ["iommu-dual" "iommu-dual-long"]
  let dual_generations = if $scene == "iommu-dual-long" { 1024 } else { 1 }
  let soak_generations = if $scene == "iommu-soak-long" { 1024 } else { 128 }
  let confirmation = $scene in ["iommu-fail-invalidate" "iommu-fail-reset" "iommu-fail-release" "iommu-fail-reference"]
  let guard = ($scene | str starts-with "iommu-guard-")
  let guard_target = if $guard { $scene | str replace "iommu-guard-" "" } else { "" }
  let daemon_rtc = $scene == "daemon-rtc"
  let daemon_target = $scene == "daemon-target"
  let daemon_stop = $scene == "daemon-stop"
  let daemon = $daemon_rtc or $daemon_target or $daemon_stop
  let evidence = $retirement or $confirmation or $hardware or $readback or $soak or $dual or $guard or $daemon
  let retire_mode = if $scene == "iommu-forced" { "forced" } else { "ruin" }
  {name: $scene, retirement: $retirement, hardware: $hardware, readback: $readback, soak: $soak, dual: $dual, dual_generations: $dual_generations, soak_generations: $soak_generations, confirmation: $confirmation, guard: $guard, guard_target: $guard_target, daemon_rtc: $daemon_rtc, daemon_target: $daemon_target, daemon_stop: $daemon_stop, daemon: $daemon, evidence: $evidence, retire_mode: $retire_mode}
}
