"""Measures groups of processes over a window, from macOS's own per-process counters
(proc_pid_rusage). Usage:

    measure.py LABEL SECONDS GROUP=PID,PID ...

Prints one line per group: CPU as a percentage of one core (the processes' own time
plus children they reaped during the window), idle wake-ups per second, and physical
footprint at the end of the window in MB. A group may be empty.
"""

import ctypes
import sys
import time

USAGE_INFO_V2 = 2


class RusageInfoV2(ctypes.Structure):
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [
        (name, ctypes.c_uint64)
        for name in (
            "user_time", "system_time", "pkg_idle_wkups", "interrupt_wkups", "pageins",
            "wired_size", "resident_size", "phys_footprint", "proc_start_abstime",
            "proc_exit_abstime", "child_user_time", "child_system_time",
            "child_pkg_idle_wkups", "child_interrupt_wkups", "child_pageins",
            "child_elapsed_abstime", "diskio_bytesread", "diskio_byteswritten",
        )
    ]


class Timebase(ctypes.Structure):
    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]


libc = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
timebase = Timebase()
libc.mach_timebase_info(ctypes.byref(timebase))
# Usage times are mach ticks; the timebase turns them into nanoseconds.
TICK_SECONDS = timebase.numer / timebase.denom / 1e9


def sample(pid):
    info = RusageInfoV2()
    if libc.proc_pid_rusage(pid, USAGE_INFO_V2, ctypes.byref(info)) != 0:
        return None
    return {
        "cpu": (info.user_time + info.system_time + info.child_user_time
                + info.child_system_time) * TICK_SECONDS,
        "wakeups": info.pkg_idle_wkups + info.interrupt_wkups
        + info.child_pkg_idle_wkups + info.child_interrupt_wkups,
        "footprint_mb": info.phys_footprint / 2**20,
    }


def main():
    label, seconds = sys.argv[1], float(sys.argv[2])
    groups = {}
    for arg in sys.argv[3:]:
        name, _, pids = arg.partition("=")
        groups[name] = [int(pid) for pid in pids.split(",") if pid]
    pids = sorted({pid for members in groups.values() for pid in members})
    before = {pid: sample(pid) for pid in pids}
    time.sleep(seconds)
    after = {pid: sample(pid) for pid in pids}
    for name, members in groups.items():
        cpu = wakeups = footprint = 0.0
        alive = 0
        for pid in members:
            first, last = before.get(pid), after.get(pid)
            if first is None or last is None:
                continue
            alive += 1
            cpu += last["cpu"] - first["cpu"]
            wakeups += last["wakeups"] - first["wakeups"]
            footprint += last["footprint_mb"]
        print(f"{label:<34} {name:<10} x{alive:<3} cpu {100 * cpu / seconds:6.1f}%  "
              f"wakeups/s {wakeups / seconds:7.1f}  mem {footprint:7.0f} MB", flush=True)


main()
