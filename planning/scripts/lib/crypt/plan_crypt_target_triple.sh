#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# plan_crypt_target_triple — the Rust target triple for this machine, or
# nothing plus exit 1 on a platform the house target list does not cover.
#
# The five rows are exactly the ones rust-development-guidelines.md section 4
# declares, so a machine outside them has no shipped binary to find and must
# fall through to the shell rungs rather than probing a directory that will
# never exist. Linux maps to musl because that is the only Linux row we build:
# a musl binary is static, so it runs on a glibc host too.
plan_crypt_target_triple() {
    local os arch
    os="$(uname -s 2>/dev/null || printf 'unknown')"
    arch="$(uname -m 2>/dev/null || printf 'unknown')"
    case "$os" in
        Linux)
            case "$arch" in
                x86_64 | amd64) printf 'x86_64-unknown-linux-musl\n' ;;
                aarch64 | arm64) printf 'aarch64-unknown-linux-musl\n' ;;
                *) return 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) printf 'x86_64-apple-darwin\n' ;;
                arm64 | aarch64) printf 'aarch64-apple-darwin\n' ;;
                *) return 1 ;;
            esac
            ;;
        # Git Bash, MSYS2 and Cygwin all run the msvc binary; there is no
        # separate Cygwin row, per the same section's tier-3 rule.
        MINGW* | MSYS* | CYGWIN*)
            case "$arch" in
                x86_64 | amd64) printf 'x86_64-pc-windows-msvc\n' ;;
                *) return 1 ;;
            esac
            ;;
        *) return 1 ;;
    esac
    return 0
}
