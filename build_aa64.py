#!/usr/bin/env python3

import argparse
import os
import shutil
import sys
import subprocess as sp
from pathlib import Path

ARCH = "aarch64"
TARGET = ARCH + "-unknown-uefi"
CONFIG = "debug"
QEMU = "qemu-system-" + ARCH

WORKSPACE_DIR = Path(__file__).resolve().parents[0]
BUILD_DIR = WORKSPACE_DIR / "build"
CARGO_BUILD_DIR = WORKSPACE_DIR / "target" / TARGET / CONFIG

OVMF_FW = WORKSPACE_DIR / "qemu_boot" / "edk2-aarch64-code.fd"
OVMF_VARS = WORKSPACE_DIR / "qemu_boot" / "edk2-arm-vars.fd"
OVMF_BIOS = WORKSPACE_DIR / "qemu_boot" / "QEMU_EFI_aarch64.fd"

def run_build(*flags):
  "Run Cargo-<tool> with the given arguments"

  cmd = ["cargo", "build", "--target", TARGET, *flags]
  sp.run(cmd).check_returncode()

def build_command():
  "Builds UEFI application"

  run_build("--package", "iron")

  # Create build folder
  boot_dir = BUILD_DIR / "EFI" / "BOOT"
  boot_dir.mkdir(parents=True, exist_ok=True)

  # Copy the build EFI application to the build directory
  built_file = CARGO_BUILD_DIR / "iron.efi"
  output_file = boot_dir / "BOOTAA64.efi"
  shutil.copy2(built_file, output_file)

  # Write a startup script to make UEFI Shell load into
  # the application automatically
  startup_file = open(BUILD_DIR / "startup.nsh", "w")
  startup_file.write("\EFI\BOOT\BOOTAA64.EFI")
  startup_file.close()

def run_command():
  "Run the application in QEMU"

  qemu_flags = [
    # Disable default devices
    # QEMU by default enables a ton of devices which slow down boot.
    "-nodefaults",

    # Select our aarch64 machine
    "-machine", "virt",
    "-cpu", "cortex-a76",
  
    # Allocate some memory
    "-m", "512M",

    # Set up semihosting
    "-semihosting",
    
    # Set up OVMF
    #"-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_FW}",
    #"-drive", f"if=pflash,format=raw,file={OVMF_VARS}",
    "-bios", OVMF_BIOS,
    # Mount a local directory as a FAT partition
    "-drive", f"format=raw,file=fat:rw:{BUILD_DIR}",

    # graphics
    "-device", "virtio-gpu-pci",

    # Enable serial
    #
    # Connect the serial port to the host. OVMF is kind enough to connect
    # the UEFI stdout and stdin to that port too.
    "-serial", "stdio",

    # Setup monitor
    "-monitor", "vc:1024x768",
  ]

  sp.run([QEMU] + qemu_flags).check_returncode()

def main(args):
  "Runs the user-requested actions"

  # Clear any Rust flags which might affect the build.
  os.environ["RUSTFLAGS"] = ""
  os.environ["RUST_TARGET_PATH"] = str(WORKSPACE_DIR)

  usage = "%(prog)s verb [options]"
  desc = "Build script for the UEFI App"

  parser = argparse.ArgumentParser(usage=usage, description=desc)

  subparsers = parser.add_subparsers(dest="verb")
  build_parser = subparsers.add_parser("build")
  run_parser = subparsers.add_parser("run")

  opts = parser.parse_args()

  if opts.verb == "build":
    build_command()
  elif opts.verb == "run":
    run_command()
  else:
    print(f"Unknown verb '{opts.verb}'")

if __name__ == '__main__':
    sys.exit(main(sys.argv))