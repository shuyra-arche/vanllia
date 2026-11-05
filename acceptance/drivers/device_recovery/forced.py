"""Exercise the production idle deadline with a real outstanding block read."""

from acceptance.drivers.device_recovery.ruin import run as retire


def run(conn, capture):
    retire(conn, capture, mode="forced")
