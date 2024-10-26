# Rusty Mouse

This project is still work in progress. Currently only the correct values are in the `.inf` file and the WDF device is being created.

## Getting started

### Prerequisites

Follow the instructions at https://github.com/microsoft/Windows-rust-driver-samples, basically:

* If you have Visual Studio installed, install _MSVC Buildtools (latest)_ and _Windows SDK_ matching your Windows under test.
* Install Clang `winget install LLVM.LLVM`.
* Install Rust.
* Make sure, you have permission to create symlinks (see https://github.com/Schniz/fnm/issues/338#issuecomment-733043042).

### Test

* Install the certificate to `Local Machine\Trusted Root Certification Authorities` and to `Local Machine\Trusted Publishers`.
* Start [DebugView](https://live.sysinternals.com/Dbgview.exe) and Enable _Capture Kernel_ and _Verbose Kernel Output_.
