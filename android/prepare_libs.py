#!/usr/bin/env python3
import os
import sys
import urllib.request
import gzip
import subprocess
import glob

MIHOMO_VERSION = "v1.19.33"
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
JNI_LIBS_DIR = os.path.join(SCRIPT_DIR, "app", "src", "main", "jniLibs")
TUN_DIR = os.path.join(SCRIPT_DIR, "tun2socks_jni")

ARCH_CONFIGS = [
    {
        "abi": "arm64-v8a",
        "mihomo_name": "mihomo-android-arm64",
        "goarch": "arm64",
        "goarm": "",
        "ndk_cc": "aarch64-linux-android24-clang"
    },
    {
        "abi": "armeabi-v7a",
        "mihomo_name": "mihomo-android-armv7",
        "goarch": "arm",
        "goarm": "7",
        "ndk_cc": "armv7a-linux-androideabi24-clang"
    },
    {
        "abi": "x86_64",
        "mihomo_name": "mihomo-android-amd64",
        "goarch": "amd64",
        "goarm": "",
        "ndk_cc": "x86_64-linux-android24-clang"
    }
]

def find_ndk_compiler(cc_name):
    # Search ANDROID_NDK_HOME, ANDROID_NDK_ROOT, or standard SDK paths
    ndk_dirs = []
    for env_var in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME"]:
        if os.environ.get(env_var):
            ndk_dirs.append(os.environ[env_var])
    
    sdk_root = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if not sdk_root and sys.platform == "win32":
        sdk_root = os.path.join(os.environ.get("LOCALAPPDATA", ""), "Android", "Sdk")
    elif not sdk_root:
        sdk_root = os.path.expanduser("~/Android/Sdk")
    
    if sdk_root and os.path.isdir(os.path.join(sdk_root, "ndk")):
        for entry in os.listdir(os.path.join(sdk_root, "ndk")):
            ndk_dirs.append(os.path.join(sdk_root, "ndk", entry))

    is_win = sys.platform == "win32"
    for ndk in ndk_dirs:
        toolchains = glob.glob(os.path.join(ndk, "toolchains", "llvm", "prebuilt", "*", "bin"))
        for tc in toolchains:
            ext = ".cmd" if is_win else ""
            candidate = os.path.join(tc, cc_name + ext)
            if os.path.isfile(candidate):
                return candidate
    return None

def download_mihomo(cfg):
    abi_dir = os.path.join(JNI_LIBS_DIR, cfg["abi"])
    os.makedirs(abi_dir, exist_ok=True)
    target_so = os.path.join(abi_dir, "libmihomo.so")
    if os.path.isfile(target_so) and os.path.getsize(target_so) > 10_000_000:
        print(f"[OK] {cfg['abi']}/libmihomo.so already exists.")
        return

    gz_name = f"{cfg['mihomo_name']}-{MIHOMO_VERSION}.gz"
    url = f"https://github.com/MetaCubeX/mihomo/releases/download/{MIHOMO_VERSION}/{gz_name}"
    print(f"Downloading Mihomo core for {cfg['abi']} from {url}...")
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req) as resp:
        gz_data = resp.read()
    decompressed = gzip.decompress(gz_data)
    with open(target_so, "wb") as f:
        f.write(decompressed)
    print(f"[Done] Saved {target_so} ({len(decompressed)} bytes)")

def build_tun2socks(cfg):
    abi_dir = os.path.join(JNI_LIBS_DIR, cfg["abi"])
    os.makedirs(abi_dir, exist_ok=True)
    target_so = os.path.join(abi_dir, "libtun2socks.so")
    if os.path.isfile(target_so) and os.path.getsize(target_so) > 5_000_000:
        print(f"[OK] {cfg['abi']}/libtun2socks.so already exists.")
        return

    cc = find_ndk_compiler(cfg["ndk_cc"])
    if not cc:
        print(f"[WARN] NDK compiler {cfg['ndk_cc']} not found. Skipping tun2socks build for {cfg['abi']}.")
        return

    print(f"Building tun2socks for {cfg['abi']} using {cc}...")
    env = os.environ.copy()
    env["CGO_ENABLED"] = "1"
    env["GOOS"] = "android"
    env["GOARCH"] = cfg["goarch"]
    if cfg["goarm"]:
        env["GOARM"] = cfg["goarm"]
    env["CC"] = cc

    cmd = ["go", "build", "-buildmode=c-shared", "-trimpath", "-ldflags", "-s -w", "-o", target_so, "."]
    res = subprocess.run(cmd, cwd=TUN_DIR, env=env)
    if res.returncode == 0:
        print(f"[Done] Built {target_so} ({os.path.getsize(target_so)} bytes)")
    else:
        print(f"[ERROR] Failed to build {target_so}")
        sys.exit(1)

def main():
    print(f"=== Preparing native libraries for Everty MultiLink Android ===")
    for cfg in ARCH_CONFIGS:
        download_mihomo(cfg)
        build_tun2socks(cfg)
    print("=== All native libraries prepared successfully ===")

if __name__ == "__main__":
    main()
