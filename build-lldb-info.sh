#!/bin/sh

export LLVM_COMMIT="738147a61d3b60ddb8994cb742999468a1e329e6"
export DEBUGSERVER_LLVM_RELEASE="19.1.4"
export BUILD_CONFIG_VERSION=2
LLVM_COMMIT_SHORT="$(echo "$LLVM_COMMIT" | head -c 8)"
export LLVM_COMMIT_SHORT
