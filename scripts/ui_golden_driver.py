#!/usr/bin/env python3
"""Drive a running Korangar client through the golden-screenshot scenarios.

The driver is started by ``scripts/ui-golden.sh`` after the client has been
launched. It talks to the client only through the X11 server (XTEST input) and
through the window-rectangle dump that the client writes when
``KORANGAR_UI_RECTS_DUMP`` is set, so it does not need any in-client hooks
beyond that dump.

For every requested variant it

1. logs in (typing the credentials into the login window),
2. confirms the server selection,
3. selects the first character,
4. waits for the in-game interface,
5. opens the windows of the scenario one after another and captures them,
6. writes ``capture.json`` with the window rectangle of every screenshot so
   the comparison script can crop exactly the window and ignore the animated
   3D world behind it.

The script never talks to the game server itself; everything goes through the
client like a real player. Credentials are read from the environment
(``UI_GOLDEN_ACCOUNT`` / ``UI_GOLDEN_PASSWORD``) and are never written to the
repository.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

try:
    from Xlib import X, XK, display
    from Xlib.ext import xtest
except ImportError:  # pragma: no cover - clear error instead of a traceback
    sys.exit("python-xlib is required for the golden UI driver (pip install python-xlib)")


# ---------------------------------------------------------------------------
# X11 input
# ---------------------------------------------------------------------------


class X11Input:
    def __init__(self, display_name: str, window_name: str = "Korangar") -> None:
        self.display = display.Display(display_name)
        self.root = self.display.screen().root
        self.window = self._find_window(self.root, window_name)
        if self.window is None:
            raise RuntimeError(f"could not find an X11 window containing {window_name!r} on {display_name}")
        self.focus()
        self.screen_width = self.display.screen().width_in_pixels
        self.screen_height = self.display.screen().height_in_pixels

    @staticmethod
    def _find_window(window: Any, name: str) -> Any:
        try:
            children = window.query_tree().children
        except Exception:
            return None
        for child in children:
            try:
                child_name = child.get_wm_name()
            except Exception:
                child_name = None
            if child_name and name in child_name:
                return child
            found = X11Input._find_window(child, name)
            if found is not None:
                return found
        return None

    def focus(self) -> None:
        """Give the client window the X11 input focus.

        Xvfb has no window manager, so nothing assigns the focus automatically;
        without this every synthetic key press would go to the root window. The
        window can briefly reject the focus request right after it is created,
        so retry until the server reports the window as focused.
        """
        self.display.set_error_handler(lambda *_: None)
        for _ in range(100):
            try:
                attributes = self.window.get_attributes()
                if attributes.map_state != X.IsViewable:
                    time.sleep(0.1)
                    continue
                self.window.set_input_focus(X.RevertToParent, X.CurrentTime)
                self.display.sync()
                if self.display.get_input_focus().focus.id == self.window.id:
                    self.display.set_error_handler(None)
                    return
            except Exception:
                pass
            time.sleep(0.1)
        self.display.set_error_handler(None)
        raise RuntimeError(f"could not focus the X11 window {self.window.id:#x}")

    def move(self, x: float, y: float) -> None:
        self.root.warp_pointer(int(round(x)), int(round(y)))
        self.display.sync()
        time.sleep(0.05)

    def click(self, x: float, y: float, button: int = 1) -> None:
        self.move(x, y)
        xtest.fake_input(self.display, X.ButtonPress, button)
        self.display.sync()
        time.sleep(0.03)
        xtest.fake_input(self.display, X.ButtonRelease, button)
        self.display.sync()
        time.sleep(0.25)

    def _keycode(self, keysym_name: str) -> int:
        keysym = XK.string_to_keysym(keysym_name)
        if keysym == 0:
            raise ValueError(f"unknown keysym {keysym_name!r}")
        return self.display.keysym_to_keycode(keysym)

    def key(self, keysym_name: str, *, shift: bool = False, alt: bool = False, control: bool = False) -> None:
        modifiers = []
        if control:
            modifiers.append("Control_L")
        if alt:
            modifiers.append("Alt_L")
        if shift:
            modifiers.append("Shift_L")
        modifier_codes = [self._keycode(name) for name in modifiers]

        for code in modifier_codes:
            xtest.fake_input(self.display, X.KeyPress, code)
        self.display.sync()
        # Modifiers have to be down across a frame boundary: the client samples
        # the modifier state when it processes the key press, and it runs at
        # 30 FPS in the headless profile.
        if modifier_codes:
            time.sleep(0.12)
        code = self._keycode(keysym_name)
        xtest.fake_input(self.display, X.KeyPress, code)
        self.display.sync()
        time.sleep(0.08)
        xtest.fake_input(self.display, X.KeyRelease, code)
        self.display.sync()
        if modifier_codes:
            time.sleep(0.08)
        for code in reversed(modifier_codes):
            xtest.fake_input(self.display, X.KeyRelease, code)
        self.display.sync()
        time.sleep(0.05)

    _SHIFTED = {
        "!": "1",
        "@": "2",
        "#": "3",
        "$": "4",
        "%": "5",
        "^": "6",
        "&": "7",
        "*": "8",
        "(": "9",
        ")": "0",
        "_": "minus",
        "+": "equal",
        ":": "semicolon",
        '"': "apostrophe",
        "<": "comma",
        ">": "period",
        "?": "slash",
        "~": "grave",
    }

    def type_text(self, text: str) -> None:
        for character in text:
            if character.isupper():
                self.key(character.lower(), shift=True)
            elif character in self._SHIFTED:
                self.key(self._SHIFTED[character], shift=True)
            elif character == " ":
                self.key("space")
            else:
                self.key(character)


# ---------------------------------------------------------------------------
# Window rectangles
# ---------------------------------------------------------------------------


@dataclass
class Rect:
    name: str
    x: float
    y: float
    width: float
    height: float

    def as_dict(self) -> dict[str, float]:
        return {"x": self.x, "y": self.y, "width": self.width, "height": self.height}

    def center(self) -> tuple[float, float]:
        return self.x + self.width / 2.0, self.y + self.height / 2.0

    def at(self, relative_x: float, relative_y: float) -> tuple[float, float]:
        return self.x + self.width * relative_x, self.y + self.height * relative_y


def read_rects(rects_file: Path) -> dict[str, Rect]:
    """Read the window rectangle dump written by the client.

    The writer replaces the file atomically, so a read either sees the previous
    or the complete new state, never a partial line.
    """
    try:
        content = rects_file.read_text()
    except FileNotFoundError:
        return {}

    rects: dict[str, Rect] = {}
    for line in content.splitlines():
        parts = line.split()
        if len(parts) != 5:
            continue
        try:
            x, y, width, height = (float(value) for value in parts[1:])
        except ValueError:
            continue
        rects[parts[0]] = Rect(parts[0], x, y, width, height)
    return rects


def wait_for_rect(
    rects_file: Path,
    names: tuple[str, ...],
    timeout: float,
    rects: dict[str, Rect] | None = None,
    *,
    predicate=None,
):
    """Wait until the dump contains one of ``names`` (optionally matching a predicate)."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        current = read_rects(rects_file)
        for name in names:
            rect = current.get(name)
            if rect is not None and (predicate is None or predicate(current)):
                return current, rect
        time.sleep(0.1)
    expected = " or ".join(names)
    raise TimeoutError(f"window {expected!r} did not appear within {timeout:.0f}s")


def wait_until(predicate, timeout: float, description: str):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.1)
    raise TimeoutError(f"timed out after {timeout:.0f}s waiting for {description}")


# ---------------------------------------------------------------------------
# Screenshots
# ---------------------------------------------------------------------------


def capture(display_name: str, target: Path) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["import", "-display", display_name, "-window", "root", str(target)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


#: Inset applied to every window crop.
#:
#: The window rectangle from the client tracks the layout size; the rendered
#: window can be a few pixels smaller at the edges (shadow/anti-aliasing), and
#: those edge pixels still show the animated 3D world behind the window. Since
#: the world must never be part of the comparison, every crop is inset a little.
CROP_INSET = 4


def capture_cropped(source: Path, target: Path, rect: Rect, screen_size: tuple[int, int]) -> dict[str, int]:
    """Crop the window out of a full screenshot.

    ImageMagick's ``convert`` is used instead of Pillow so the harness keeps
    working with the tools that the existing verification scripts already use.
    """
    x = max(0, int(round(rect.x)) + CROP_INSET)
    y = max(0, int(round(rect.y)) + CROP_INSET)
    width = min(int(round(rect.width)) - 2 * CROP_INSET, screen_size[0] - x)
    height = min(int(round(rect.height)) - 2 * CROP_INSET, screen_size[1] - y)
    width = max(1, width)
    height = max(1, height)
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["magick", str(source), "-crop", f"{width}x{height}+{x}+{y}", "+repage", str(target)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return {"x": x, "y": y, "width": width, "height": height}


# ---------------------------------------------------------------------------
# Scenario
# ---------------------------------------------------------------------------


@dataclass
class Screen:
    """One screenshot of the golden matrix."""

    name: str
    #: Window whose rectangle is captured and compared.
    window: str | None
    #: Keys pressed before capturing (each a tuple of X11 keysym name + flags).
    keys: list[tuple[str, dict]] = field(default_factory=list)
    #: Keys pressed after capturing to return to the previous state.
    toggle_keys: list[tuple[str, dict]] = field(default_factory=list)
    #: Extra rectangles to exclude from the comparison (animation masks).
    masks: list[tuple[float, float, float, float]] = field(default_factory=list)


IN_GAME_SCREENS = [
    Screen(
        name="inventory",
        window="InventoryWindow",
        keys=[("i", {})],
        toggle_keys=[("i", {})],
    ),
    Screen(
        name="equipment",
        window="EquipmentWindow",
        keys=[("q", {"alt": True})],
        toggle_keys=[("q", {"alt": True})],
        # The paper doll plays the idle animation; the comparison mask computed
        # from two consecutive frames already covers it, this is a safety net
        # for the character area of the window (left third, lower two thirds).
        masks=[(0.0, 0.30, 0.40, 0.75)],
    ),
    Screen(
        name="character_overview",
        window="CharacterOverviewWindow",
        keys=[],
    ),
]


def run_scenario(
    args: argparse.Namespace,
    x11: X11Input,
    rects_file: Path,
    output_directory: Path,
    metadata: dict[str, Any],
) -> None:
    screen_size = (x11.screen_width, x11.screen_height)
    captures: list[dict[str, Any]] = []

    def save(name: str, window: str, masks=None) -> None:
        rects = read_rects(rects_file)
        rect = rects.get(window)
        if rect is None:
            raise RuntimeError(f"window {window!r} is not in the rectangle dump ({sorted(rects)})")

        # Other windows can overlap the captured window (the always-open chat
        # and hotbar overlays are drawn on top of the character overview, for
        # example). Their pixels are dynamic, so mask every part of the capture
        # that is covered by another window rectangle.
        # The dump lists windows from the bottom to the top of the z-order, so
        # only windows that come after the captured one can cover it.
        names = list(rects)
        target_index = names.index(window)
        overlap_masks = []
        for other_name in names[target_index + 1:]:
            other = rects[other_name]
            left = max(rect.x, other.x)
            top = max(rect.y, other.y)
            right = min(rect.x + rect.width, other.x + other.width)
            bottom = min(rect.y + rect.height, other.y + other.height)
            if right - left <= 0 or bottom - top <= 0:
                continue
            overlap_masks.append(
                [
                    (left - rect.x) / rect.width,
                    (top - rect.y) / rect.height,
                    (right - left) / rect.width,
                    (bottom - top) / rect.height,
                ]
            )
        full_path = output_directory / f"{name}.full.png"
        capture(args.display, full_path)
        cropped_path = output_directory / f"{name}.png"
        crop = capture_cropped(full_path, cropped_path, rect, screen_size)
        # A second capture a moment later is used by the compare/baseline step
        # to derive an animation mask: everything that changed between the two
        # frames is animated and must be ignored when comparing.
        # The second capture is used for the animation mask; crop it exactly like
        # the first one so the two images can be diffed.
        second_full_path = output_directory / f"{name}.second.full.png"
        second_path = output_directory / f"{name}.second.png"
        time.sleep(args.animation_interval)
        capture(args.display, second_full_path)
        capture_cropped(second_full_path, second_path, rect, screen_size)
        captures.append(
            {
                "name": name,
                "window": window,
                "rect": crop,
                "masks": (masks or []) + overlap_masks,
                "screen": {"width": screen_size[0], "height": screen_size[1]},
            }
        )
        print(f"  captured {name} ({window} at {crop['x']},{crop['y']} {crop['width']}x{crop['height']})", flush=True)

    # --- login ---------------------------------------------------------
    wait_for_rect(rects_file, ("LoginWindow",), args.timeout)
    time.sleep(0.8)
    x11.move(screen_size[0] - 4, screen_size[1] - 4)
    time.sleep(0.3)
    save("login", "LoginWindow")

    # The harness pre-fills the remembered login credentials in
    # client/login_settings.ron, so Enter submits the login form (the client
    # supports that since the menu can be used without a mouse). This is much
    # more robust than clicking the text boxes, whose positions depend on the
    # interface scaling and the theme paddings.
    x11.key("Return")
    try:
        rects, _ = wait_for_rect(
            rects_file,
            ("ServerSelectionWindow",),
            args.login_timeout,
            predicate=lambda current: "ServerSelectionWindow" in current,
        )
    except TimeoutError:
        # Fallback: type the credentials by hand. The relative positions were
        # measured on the default theme; they are only a best effort.
        print("  login form did not submit, typing credentials instead", flush=True)
        login_rect = read_rects(rects_file)["LoginWindow"]
        x11.click(*login_rect.at(0.5, 0.48))
        x11.key("Home")
        for _ in range(64):
            x11.key("BackSpace")
        x11.type_text(args.account)
        x11.click(*login_rect.at(0.5, 0.556))
        x11.key("Home")
        for _ in range(64):
            x11.key("BackSpace")
        x11.type_text(args.password)
        time.sleep(0.3)
        x11.click(*login_rect.at(0.5, 0.893))
        wait_for_rect(rects_file, ("ServerSelectionWindow",), args.timeout)

    # --- server selection ---------------------------------------------
    # Enter confirms the server selection as well.
    time.sleep(1.0)
    x11.move(screen_size[0] - 4, screen_size[1] - 4)
    time.sleep(0.3)
    save("select_server", "ServerSelectionWindow")
    x11.key("Return")

    # --- character selection ------------------------------------------
    wait_for_rect(rects_file, ("CharacterSelectionWindow",), args.timeout)
    time.sleep(1.5)
    x11.move(screen_size[0] - 4, screen_size[1] - 4)
    time.sleep(0.3)
    save("character_select", "CharacterSelectionWindow")

    # The first character slot is the top-left card inside the window. These
    # are relative coordinates, so they scale with the interface scaling.
    character_rect = read_rects(rects_file)["CharacterSelectionWindow"]
    x11.click(*character_rect.at(0.121, 0.223))

    # --- in game -------------------------------------------------------
    print("  waiting for the map to load...", flush=True)
    wait_until(
        lambda: "CharacterOverviewWindow" in read_rects(rects_file),
        args.map_timeout,
        "the in-game interface (map load)",
    )
    # Let the map finish loading and the first entity/effect batch settle.
    time.sleep(args.map_settle)
    x11.move(screen_size[0] - 4, screen_size[1] - 4)
    time.sleep(0.5)

    for screen in IN_GAME_SCREENS:
        before = set(read_rects(rects_file))
        for keysym, flags in screen.keys:
            x11.key(keysym, **flags)
        if screen.window is not None:
            wait_for_rect(rects_file, (screen.window,), args.timeout)
            # Wait until the window has a stable size (it may be resized on the
            # first layout pass after opening).
            previous = None
            for _ in range(20):
                current = read_rects(rects_file).get(screen.window)
                if current is not None and previous == (current.x, current.y, current.width, current.height):
                    break
                previous = (current.x, current.y, current.width, current.height) if current else None
                time.sleep(0.15)
        time.sleep(1.0)
        x11.move(screen_size[0] - 4, screen_size[1] - 4)
        time.sleep(0.4)
        save(screen.name, screen.window, [list(mask) for mask in screen.masks])
        for keysym, flags in screen.toggle_keys:
            x11.key(keysym, **flags)
        time.sleep(0.6)
        del before

    payload = {
        "metadata": metadata,
        "captures": captures,
    }
    (output_directory / "capture.json").write_text(json.dumps(payload, indent=2))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--display", required=True, help="X11 display, e.g. :121")
    parser.add_argument("--rects-file", required=True, type=Path, help="file the client dumps window rectangles to")
    parser.add_argument("--output", required=True, type=Path, help="directory for the captured screenshots")
    parser.add_argument("--theme", required=True, help="theme name that is being captured")
    parser.add_argument("--scale", required=True, help="interface scaling that is being captured")
    parser.add_argument("--account", required=True, help="test account name")
    parser.add_argument("--password", required=True, help="test account password")
    parser.add_argument("--window-timeout", type=float, default=180.0, help="seconds to wait for the client X11 window")
    parser.add_argument("--timeout", type=float, default=90.0, help="seconds to wait for the next window")
    parser.add_argument("--login-timeout", type=float, default=25.0, help="seconds to wait for the server selection window")
    parser.add_argument("--map-timeout", type=float, default=120.0, help="seconds to wait for the map load")
    parser.add_argument("--map-settle", type=float, default=6.0, help="seconds to let the map settle after loading")
    parser.add_argument("--animation-interval", type=float, default=0.6, help="delay between the two animation frames")
    parser.add_argument("--window-name", default="Korangar", help="X11 window title substring")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)
    metadata = {
        "theme": args.theme,
        "scale": args.scale,
        "screen": None,
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
    }

    # The client needs a while to start under software rendering (map data and
    # shader cache), so wait for its X11 window instead of failing right away.
    deadline = time.monotonic() + args.window_timeout
    x11 = None
    while time.monotonic() < deadline:
        try:
            x11 = X11Input(args.display, args.window_name)
            break
        except RuntimeError:
            time.sleep(0.5)
    if x11 is None:
        raise TimeoutError(f"the client X11 window did not appear within {args.window_timeout:.0f}s")
    metadata["screen"] = {"width": x11.screen_width, "height": x11.screen_height}

    print(f"golden UI driver: theme={args.theme} scale={args.scale} display={args.display}", flush=True)
    run_scenario(args, x11, args.rects_file, args.output, metadata)
    print(f"golden UI driver: wrote {args.output / 'capture.json'}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except TimeoutError as error:
        print(f"golden UI driver: {error}", file=sys.stderr, flush=True)
        sys.exit(2)
