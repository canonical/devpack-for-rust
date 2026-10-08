import argparse
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import termios
import time


def read_until(master, process, marker, timeout):
    output = bytearray()
    deadline = time.monotonic() + timeout

    while marker not in output and time.monotonic() < deadline:
        if process.poll() is not None:
            break
        readable, _, _ = select.select([master], [], [], 0.2)
        if readable:
            try:
                output.extend(os.read(master, 65536))
            except OSError:
                break

    if marker not in output:
        raise RuntimeError(
            f"timed out waiting for {marker!r}; output so far:\n"
            f"{output.decode(errors='replace')[-4000:]}"
        )
    return output


def read_to_exit(master, process, timeout):
    output = bytearray()
    deadline = time.monotonic() + timeout

    while process.poll() is None and time.monotonic() < deadline:
        readable, _, _ = select.select([master], [], [], 0.2)
        if readable:
            try:
                output.extend(os.read(master, 65536))
            except OSError:
                break

    if process.poll() is None:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        raise RuntimeError(
            f"interactive installer did not finish within {timeout} seconds; "
            f"output:\n{output.decode(errors='replace')[-6000:]}"
        )

    while select.select([master], [], [], 0)[0]:
        try:
            output.extend(os.read(master, 65536))
        except OSError:
            break

    if process.returncode != 0:
        raise RuntimeError(
            f"interactive installer exited with {process.returncode}; output:\n"
            f"{output.decode(errors='replace')[-6000:]}"
        )
    return output


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="exercise the selected recipes without installing tools",
    )
    args = parser.parse_args()

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))

    def attach_terminal():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    env = os.environ.copy()
    env["TERM"] = "xterm"
    env["COLUMNS"] = "120"
    env["LINES"] = "40"

    command = [
        "sudo", "-n", "env",
        "PATH=/home/ubuntu/.cargo/bin:/snap/bin:/usr/local/sbin:/usr/local/bin:"
        "/usr/sbin:/usr/bin:/sbin:/bin",
        "snap", "run", "devpack-for-rust",
        "--override-user", "ubuntu",
        "--override-shell-type", "bash",
        "--skip-cc-check",
    ]
    if args.dry_run:
        command.extend(["--dry-run", "-vv"])

    subprocess.run(["sudo", "loginctl", "enable-linger", "ubuntu"], check=True)

    process = subprocess.Popen(
        command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
        close_fds=True,
        preexec_fn=attach_terminal,
    )
    os.close(slave)

    try:
        output = read_until(
            master, process, b"Pick a rust channel", timeout=30
        )

        # Select Rust stable, just, and ripgrep, then finish through the TUI's
        # All Done option instead of sending a process-level interrupt.
        keys = (
            b"l", b" ", b"h", b"j", b"j", b"l", b" ",
            b"h", b"j", b"l", b"j", b"j", b"l", b"h", b" ",
            b"h", b"j", b"\r",
        )
        for key in keys:
            os.write(master, key)
            readable, _, _ = select.select([master], [], [], 0.2)
            if readable:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break

        output.extend(read_to_exit(
            master, process, timeout=60 if args.dry_run else 1800
        ))
        if b"All done! Enjoy your devpack-for-rust!" not in output:
            raise RuntimeError(
                "installer did not report success; output:\n"
                f"{output.decode(errors='replace')[-6000:]}"
            )
        if args.dry_run:
            dry_run_output = output.decode(errors="replace")
            expected_messages = (
                'Using rustup to install rust channel "stable"',
                'using snap to install "just" with --classic confinement',
                'Using cargo to install "ripgrep"',
                'dry-"running" command',
            )
            for expected in expected_messages:
                if expected not in dry_run_output:
                    raise RuntimeError(
                        f"dry-run output did not contain {expected!r}; output:\n"
                        f"{dry_run_output[-6000:]}"
                    )
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        os.close(master)


if __name__ == "__main__":
    main()
