#!/usr/bin/env python3

import argparse
import os
import shutil
import sys
import subprocess as sp
from pathlib import Path

# Constants for our own use
X86 = 0
X86_64 = 1
AA64 = 2

TEST = 0
DEBUG = 1
RELEASE = 2
PROFILING = 3
OPTIMIZED = 4
DEBUG_OPTIMIZED = 5

# The amount of memory to allocate to the VM
QEMU_SYSTEM_MEMORY = "256M"

# The directory where qemu boot media images are stored
QEMU_BOOT_MEDIA_DIR = "qemu_boot"

# The name of each architecture for qemu
QEMU_ARCH_NAMES = [
  "i386",
  "x86_64",
  "aarch64"
]

# The name of each architecture for uefi
UEFI_ARCH_NAMES = [
    "i686",
    "x86_64",
    "aarch64"
]

# The name of each architecture for cargo
CARGO_ARCH_NAMES = [
    "x86",
    "x86_64",
    "aarch64",
]

# The specific boot media for each architecture
# Firmware
QEMU_OVMF_FW = [
  "edk2-i386-code.fd",
  "OVMF_CODE-pure-efi.fd",
  "UNUSED",
]
# Vars
QEMU_OVMF_VARS = [
  "edk2-i386-vars.fd",
  "OVMF_VARS-pure-efi.fd",
  "UNUSED",
]
# BIOS
QEMU_OVMF_BIOS = [
  "UNUSED",
  "UNUSED",
  "QEMU_EFI_aarch64.fd",
]

# Supported arctions, archs, and configs
SUPPORTED_ACTIONS = ["build", "run"]
SUPPORTED_ARCHS = ["x86", "x64", "aa64"]
SUPPORTED_CONFIGS = ["test", "debug", "release", "profiling", "optimized", "debug_optimized"]

# The uefi boot file names for each architecture
UEFI_BOOT_FILE = [
  "BOOT.EFI",
  "BOOTX64.EFI",
  "BOOTAA64.EFI"
]

# Our directories/paths
WORKSPACE_DIR = Path(__file__).resolve().parents[0]
BUILD_DIR = WORKSPACE_DIR / "build"
OVMF_BASE = WORKSPACE_DIR / QEMU_BOOT_MEDIA_DIR

# The common qemu command options
QEMU_BASE_FLAGS = [
  # Disable default devices
  # QEMU by default enables a ton of devices which slow down boot.
  "-nodefaults",
  
  # Allocate some memory
  "-m", QEMU_SYSTEM_MEMORY,
  
  # Mount a local directory as a FAT partition
  "-drive", f"format=raw,file=fat:rw:{BUILD_DIR}",
  
  # Enable serial
  #
  # Connect the serial port to the host. OVMF is kind enough to connect
  # the UEFI stdout and stdin to that port too.
  "-serial", "stdio",
  
  # Setup monitor
  "-monitor", "vc:1024x768",
]

# The globals that will be configured upon arch selection
current_arch_id = 0
current_config_id = 0
current_target = ""
qemu_cmd = ""
qemu_flags = []
cargo_build_dir = ""

def init_arch(arch, config):
  global current_arch_id, current_config_id
  global current_target, qemu_cmd, qemu_flags, cargo_build_dir
  
  if config == "test":
    current_config_id = TEST
    current_build_subdir = "debug"
  elif config == "debug":
    current_config_id = DEBUG
    current_build_subdir = "debug"
  elif config == "release":
    current_config_id = RELEASE
    current_build_subdir = "release"
  elif config == "profiling":
    current_config_id = PROFILING
    current_build_subdir = "profiling"
  elif config == "optimized":
    current_config_id = OPTIMIZED
    current_build_subdir = "lto-release"
  elif config == "debug_optimized":
    current_config_id = DEBUG_OPTIMIZED
    current_build_subdir = "lto-debug"

  current_target = UEFI_ARCH_NAMES[current_arch_id] + "-unknown-uefi"
  qemu_cmd = "qemu-system-" + QEMU_ARCH_NAMES[current_arch_id]
  cargo_build_dir = WORKSPACE_DIR / "target" / current_target / current_build_subdir
  
  qemu_x86_flags = [
    # Use a standard VGA for graphics
    "-vga", "std",
    
    # Use a modern machine, with acceleration if possible.
    "-machine", "pc-q35-2.10,accel=tcg",

    # Set up the CPU
    "-cpu", "max",
    
    # Set up OVMF
    "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_BASE / QEMU_OVMF_FW[current_arch_id]}",
    "-drive", f"if=pflash,format=raw,file={OVMF_BASE / QEMU_OVMF_VARS[current_arch_id]}",
    
    # Debug exit for x86/x64
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
  ]

  qemu_aa64_flags = [
    # Select our aarch64 machine
    "-machine", "virt",
    "-cpu", "cortex-a76",

    # Yes, AA64 uses a bios and not the 64MB OVMF boot disk (go figure)
    "-bios", OVMF_BASE / QEMU_OVMF_BIOS[current_arch_id],
    
    # Set up semihosting (debug exit for aa64)
    "-semihosting",
  ]
  
  if arch == "x86" or arch == "x64":
    qemu_flags = QEMU_BASE_FLAGS + qemu_x86_flags
  elif arch == "aa64":
    qemu_flags = QEMU_BASE_FLAGS + qemu_aa64_flags

def run_build(rust_flags, linker_flags, *flags):
  global current_target, cargo_build_dir
  
  "Run Cargo-<tool> with the given arguments"

  rf = "RUSTFLAGS=\""

  if rust_flags != None:
    rf = rf + rust_flags + " "

  if linker_flags != None:
    rf = rf + "-Clink-args=" + linker_flags

  rf = rf.strip() + "\" "

  if rust_flags != None or linker_flags != None:
    rf_array = [
      rf
    ]
  else:
    rf_array = []

  if current_config_id == TEST:
    cmd = rf_array + ["cargo", "test", "--target", current_target, *flags]
  elif current_config_id == DEBUG:
    cmd = rf_array + ["cargo", "build", "--target", current_target, *flags]
  elif current_config_id == RELEASE:
    cmd = rf_array + ["cargo", "build", "--release", "--target", current_target, *flags]
  elif current_config_id == PROFILING:
    cmd = rf_array + ["cargo", "build", "--profile=profiling", "--target", current_target, *flags]
  elif current_config_id == OPTIMIZED:
    cmd = rf_array + ["cargo", "build", "--profile=lto-release", "--target", current_target, *flags]
  elif current_config_id == DEBUG_OPTIMIZED:
    cmd = rf_array + ["cargo", "build", "--profile=lto-debug", "--target", current_target, *flags]

  sp.run(cmd).check_returncode()

def build_command(rust_flags, linker_flags):
  global current_arch_id, cargo_build_dir
  
  "Builds iron"

  run_build(rust_flags, linker_flags, "--package", "iron")

  # Create build folder
  boot_dir = BUILD_DIR / "EFI" / "BOOT"
  boot_dir.mkdir(parents=True, exist_ok=True)

  # Copy the build EFI application to the build directory
  built_file = cargo_build_dir / "iron.efi"
  output_file = boot_dir / UEFI_BOOT_FILE[current_arch_id]
  output_full_path = str("\EFI\BOOT\\" + UEFI_BOOT_FILE[current_arch_id])
  shutil.copy2(built_file, output_file)

  # Write a startup script to make UEFI Shell load into
  # the application automatically
  startup_file = open(BUILD_DIR / "startup.nsh", "w")
  startup_file.write(output_full_path)
  startup_file.close()

def run_command():
  global qemu_cmd, qemu_flags
  
  "Runs iron in QEMU"

  sp.run([qemu_cmd] + qemu_flags).check_returncode()

def main(args):
  global current_arch_id, current_config_id
  global current_target, qemu_cmd, qemu_flags, cargo_build_dir

  "Runs the user-requested actions"

  # Clear any Rust flags which might affect the build.
  os.environ["RUSTFLAGS"] = ""
  os.environ["RUST_TARGET_PATH"] = str(WORKSPACE_DIR)

  options = [
    "-lf, --linkerflags for specifying linker flags",
    "-rf, --rustflags for specifying rust compiler flags",
    #"-af, --asmflags for specifying assembler flags",
  ]

  usage = "%(prog)s <action> <arch> <config> [options]\n\nOptions:\n\n" + "\n".join(options) + "\n"
  desc = "Build script for iron"

  # Create the parser
  parser = argparse.ArgumentParser(usage=usage, description=desc)

  # Add the required arguments
  parser.add_argument("action", help="The action to perform", choices=["build", "run"])
  parser.add_argument("arch", help="The architecture to build for", choices=SUPPORTED_ARCHS)
  parser.add_argument("config", help="The configuration to build", choices=SUPPORTED_CONFIGS)

  # Add the optional arguments
  parser.add_argument("-lf", "--linkerflags", help="The linker flags to use")
  parser.add_argument("-rf", "--rustflags", help="The compiler flags to use")
  #parser.add_argument("-af", "--asmflags", help="The assembler flags to use")

  opts = parser.parse_args()

  if opts.action not in SUPPORTED_ACTIONS:
    print(f"Unknown action '{opts.action}'")
    print(f"Supported actions: {SUPPORTED_ACTIONS}")
    return 1
  
  if opts.arch not in SUPPORTED_ARCHS:
    print(f"Unknown architecture '{opts.arch}'")
    print(f"Supported architectures: {SUPPORTED_ARCHS}")
    return 1
  
  if opts.config not in SUPPORTED_CONFIGS:
    print(f"Unknown configuration '{opts.config}'")
    print(f"Supported configurations: {SUPPORTED_CONFIGS}")
    return 1
  
  if opts.arch == "x86":
    current_arch_id = X86
  elif opts.arch == "x64":
    current_arch_id = X86_64
  elif opts.arch == "aa64":
    current_arch_id = AA64

  init_arch(opts.arch, opts.config)

  if opts.action == "build":
    build_command(opts.rustflags, opts.linkerflags)
  elif opts.action == "run":
    run_command()
  else:
    print(f"Unknown action '{opts.action}'")

if __name__ == '__main__':
    sys.exit(main(sys.argv))