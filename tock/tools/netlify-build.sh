#!/usr/bin/env bash
#
# Script used to install additional requirements to the base Netlify image.
#
# Should not be used or relied on outside of Netlify context.
#
#  Author: Pat Pannuto <pat.pannuto@gmail.com>


set -e
set -u
set -x

curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-10-17

export PATH="$PATH:$HOME/.cargo/bin"

tools/build-all-docs.sh
