import shutil
import subprocess
import os


def get_makefile_variable(variable_name, makefile='Makefile'):
    command = ['make', '-f', makefile, f'print-{variable_name}']
    result = subprocess.run(
        command, capture_output=True, text=True, check=True)
    return result.stdout.strip().split("=", 1)[1].strip()


try:
    target = get_makefile_variable('TARGET')
    arch = get_makefile_variable('ARCH')
    print(f'Target: {target}')
    print(f'Arch: {arch}')
    runner = f'./script/{arch}/{arch}_test.sh'
    entry = f'./script/{arch}/{arch}_entry.asm'
    linker = f'./script/{arch}/linker.ld'
except subprocess.CalledProcessError as e:
    print(f"Error reading Makefile variable: {e}")

if not os.path.exists('.cargo'):
    os.mkdir('.cargo')

with open('.cargo/config.toml', 'w') as f:
    config = f"""
    [build]
    target = "{target}"

    [target.'cfg(target_os = "none")']
    runner = "{runner}"
    rustflags = [
        "-Clink-arg=-Tsel4_ipc/linker.ld",
        "-Cforce-frame-pointers=yes",
        '--cfg=board="qemu"',
    ]
    """
    f.write(config)

shutil.copyfile(entry, 'src/entry.asm')
shutil.copyfile(linker, 'linker.ld')
