#!/usr/bin/env nu

# Discover the inputs of one normal boot: the product initrd and the Kernel ELF.
#
#   nu scripts/build.nu <profile>    # build product initrd + Kernel, print record
#   nu scripts/build.nu --plan       # print planned commands, run nothing
#
# The record is the only stdout line on success and the only path-discovery
# authority. Progress and cargo logs go to stderr. A failed build exits with the
# build's own code and prints no success record.

const SCENE = "product"
const TARGET = "riscv64gc-unknown-none-elf"

def cargo-product [profile: string] {
  ["cargo" "image" "build" "product" $profile]
}

def cargo-kernel [root: string, profile: string] {
  let release = if $profile == "release" { ["--release"] } else { [] }
  [
    "cargo" "build"
    "--locked"
    "--manifest-path" ($root | path join "components/kernel/Cargo.toml")
    "--bin" "shuyra"
    "--target" $TARGET
    "--features" "semihosting"
    "--target-dir" ($root | path join "target/kernel")
    "--message-format=json-render-diagnostics"
    ...$release
  ]
}

def last-ok [stdout: string] {
  let results = ($stdout | lines | where { |l| $l =~ '^ok\s+' })
  if ($results | is-empty) {
    error make {msg: "cargo image build 没有打印以 ok 开头的结果行"}
  }
  let found = ($results | last)
  let at = ($found | str replace --regex '^ok\s+' '' | str trim | path expand)
  if not ($at | path exists) {
    error make {msg: $"cargo image build 说落在 ($at)，但它不存在"}
  }
  if ($at | path type) != "file" {
    error make {msg: $"cargo image build 的结果不是普通文件：($at)"}
  }
  $at
}

def kernel-elf [stdout: string] {
  let artifacts = (
    $stdout
    | lines
    | each { |l|
        if $l =~ '^\s*\{' { try { $l | from json } catch { null } } else { null }
      }
    | where { |m| ($m != null) and ($m.reason? == "compiler-artifact") }
    | where { |m| $m.target?.name? == "shuyra" }
    | where { |m| ($m.target?.kind? | default []) | any { |k| $k == "bin" } }
    | where { |m| ($m.executable? | default "") | is-not-empty }
  )
  if ($artifacts | is-empty) {
    error make {msg: "cargo build 没有为 shuyra 的 bin 目标报告可执行文件"}
  }
  let exe = ($artifacts | last | get executable | path expand)
  if not ($exe | path exists) {
    error make {msg: $"cargo build 报告的 ELF 不存在：($exe)"}
  }
  if ($exe | path type) != "file" {
    error make {msg: $"cargo build 报告的 ELF 不是普通文件：($exe)"}
  }
  $exe
}

def main [profile: string = "release", --plan] {
  let root = ($env.FILE_PWD | path dirname)
  if $profile not-in ["debug" "release"] {
    error make {msg: $"profile 只接受 debug 或 release，得到 ($profile)"}
  }

  if $plan {
    return ({
      scene: $SCENE
      profile: $profile
      cwd: $root
      product: (cargo-product $profile)
      kernel: (cargo-kernel $root $profile)
    } | to json --raw)
  }

  cd $root
  print -e $"build: product ($profile) cwd=($root)"
  let image = (^cargo ...((cargo-product $profile) | skip 1) | complete)
  print -n -e $image.stderr
  print -n -e $image.stdout
  if $image.exit_code != 0 {
    exit $image.exit_code
  }
  let initrd = (last-ok $image.stdout)

  print -e $"build: kernel ($profile) cwd=($root)"
  let kernel = (^cargo ...((cargo-kernel $root $profile) | skip 1) | complete)
  print -n -e $kernel.stderr
  if $kernel.exit_code != 0 {
    print -n -e $kernel.stdout
    exit $kernel.exit_code
  }
  let elf = (kernel-elf $kernel.stdout)

  {
    scene: $SCENE
    profile: $profile
    initrd: $initrd
    kernel: $elf
  } | to json --raw
}
