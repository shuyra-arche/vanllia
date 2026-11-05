"""Own the serial listener, repeating input and explicit driver extensions.

Built-in drivers use normal package imports. ``--feed-script`` is an explicit
user extension; its isolated loader retains relative imports for that extension
only. It is never used to discover or import Vanllia's own test modules.
"""

import argparse
import importlib
import importlib.util
from pathlib import Path
import socket
import sys
import threading
import time


def load_driver(module="", script=""):
    if module and script:
        raise ValueError("choose a package driver or an external script")
    if module:
        if not module.startswith("acceptance.drivers."):
            raise ValueError("built-in driver must belong to acceptance.drivers")
        return importlib.import_module(module)
    if script:
        # Explicit external extension, not an internal package loading scheme.
        path = Path(script).resolve(strict=True)
        spec = importlib.util.spec_from_file_location(
            "vanllia_external_driver", path,
            submodule_search_locations=[str(path.parent)],
        )
        driver = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = driver
        spec.loader.exec_module(driver)
        return driver
    return None


def capture_output(conn, capture):
    while True:
        try:
            data = conn.recv(65536)
        except OSError:
            return
        if not data:
            return
        with open(capture, "ab") as output:
            output.write(data)


def serve(port_file, capture, text, after=5.0, module="", script=""):
    driver = load_driver(module)
    if script and not Path(script).is_file():
        raise ValueError("external script must name a regular file")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind(("127.0.0.1", 0))
        server.listen(8)
        Path(port_file).write_text(str(server.getsockname()[1]))
        while True:
            conn, _ = server.accept()
            with open(capture, "ab") as output:
                output.write("\n──── 一例 ────\n".encode())
            if driver or script:
                result = Path(str(capture) + ".result")
                try:
                    # Legacy extensions were loaded afresh for each connection.
                    # Built-ins retain normal package identity; no fake packages.
                    selected = load_driver(script=script) if script else driver
                    selected.run(conn, capture)
                    result.write_text("ok")
                except Exception as error:
                    result.write_text(str(error))
                    raise
                finally:
                    conn.close()
                continue
            threading.Thread(target=capture_output, args=(conn, capture), daemon=True).start()
            time.sleep(after)
            try:
                while True:
                    conn.sendall((text + "\n").encode())
                    time.sleep(2)
            except OSError:
                pass
            finally:
                conn.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port_file")
    parser.add_argument("capture")
    parser.add_argument("text")
    parser.add_argument("after", type=float, nargs="?", default=5.0)
    choice = parser.add_mutually_exclusive_group()
    choice.add_argument("--driver", default="")
    choice.add_argument("--script", default="")
    args = parser.parse_args()
    if args.after < 0:
        parser.error("after must be nonnegative")
    serve(args.port_file, args.capture, args.text, args.after, args.driver, args.script)


if __name__ == "__main__":
    sys.dont_write_bytecode = True
    main()
