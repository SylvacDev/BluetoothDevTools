"""The SDK's GUI example, in Python with tkinter. Choose a dongle, start the scan, connect an
instrument through the chosen dongle, read its values, send it a command.

    python bluetooth_example.py

A tkinter program touches its widgets from the tkinter thread only, so the events are taken
every 50 ms (`after`) with `next_event(0)`, which never blocks.
"""

from __future__ import annotations

import math
import tkinter as tk
from dataclasses import dataclass, field
from tkinter import font, ttk

import sylvac_ble
from sylvac_ble import (
    AdvertiserChanged,
    AdvertiserRemoved,
    BleManager,
    CentralAdded,
    CentralRemoved,
    ConnectionStep,
    ConnectionStepChanged,
    Event,
    InstrumentButtonPressed,
    InstrumentResponseReceived,
    LibLog,
    MeasurementParamsChanged,
    NumericalValueChanged,
    PeripheralAdded,
    PeripheralRemoved,
    Rejected,
)

POLL_MS = 50
# Events handled per poll, so a burst cannot freeze the window.
BATCH = 200
# The status dot: subscribed, connecting, disconnected.
GREEN, ORANGE, RED = "green", "orange", "red"


def display_name(member: object) -> str:
    """An enumeration member as the window shows it: `SdsUnit.MILLIMETER` -> `Millimeter`."""
    return str(getattr(member, "name", member)).title().replace("_", "")


def f3(value: float) -> str:
    """Three decimals; NaN before the first value."""
    return "NaN" if math.isnan(value) else f"{value:.3f}"


@dataclass
class InstrumentRow:
    """A line of "Connected instruments": one instrument."""

    line: ttk.Frame
    dot: tk.Canvas
    widgets: dict[str, ttk.Label] = field(default_factory=dict)


@dataclass
class AdvertiserRow:
    """A line of "Scan": one device heard."""

    frame: ttk.Frame
    name: ttk.Label
    value: ttk.Label


class App:
    def __init__(self, root: tk.Tk) -> None:
        self.root = root
        root.title("BluetoothExample")
        root.geometry("800x800")
        title = font.nametofont("TkDefaultFont").copy()
        title.configure(size=15)
        self.title_font = title
        self.dongles: list[str] = []
        self.instruments: dict[str, InstrumentRow] = {}
        self.advertisers: dict[str, AdvertiserRow] = {}
        self.poll_id: str | None = None

        page = ttk.Frame(root, padding=5)
        page.pack(fill="both", expand=True)

        # Dongles view
        ttk.Label(page, text="Dongles", font=title).pack(anchor="w", pady=(0, 5))
        self.selected_dongle = tk.StringVar()
        self.dongle_box = ttk.Combobox(
            page, textvariable=self.selected_dongle, state="readonly", width=24
        )
        self.dongle_box.pack(anchor="w")

        # Connections view
        ttk.Label(page, text="Connected instruments", font=title).pack(anchor="w", pady=(10, 5))
        self.instrument_list = ttk.Frame(page)
        self.instrument_list.pack(anchor="w", fill="x")

        # Send a command to all connected instruments
        ttk.Label(page, text="Sylvac command").pack(anchor="w")
        line = ttk.Frame(page)
        line.pack(anchor="w")
        self.request_text = tk.StringVar(value="ID?")
        self.request_entry = ttk.Entry(line, textvariable=self.request_text, width=14)
        self.request_entry.pack(side="left")
        self.request_entry.bind("<Return>", lambda _: self.send_request_to_all())
        ttk.Button(line, text="Send to all", command=self.send_request_to_all).pack(
            side="left", padx=(5, 0)
        )

        # Scan view
        ttk.Label(page, text="Scan", font=title).pack(anchor="w", pady=(10, 5))
        line = ttk.Frame(page)
        line.pack(anchor="w")
        ttk.Button(line, text="Start scan", command=self.start_scan).pack(side="left")
        ttk.Button(line, text="Stop scan", command=self.stop_scan).pack(side="left", padx=(5, 0))
        ttk.Button(line, text="Clear scan", command=self.clear_scan).pack(side="left", padx=(5, 0))
        self.advertiser_list = ttk.Frame(page)
        self.advertiser_list.pack(anchor="w", fill="x", pady=(5, 0))

        root.protocol("WM_DELETE_WINDOW", self.close)
        self.manager = BleManager()
        self.poll()

    # ─── Events ──────────────────────────────────────────────────────────────

    def poll(self) -> None:
        """Takes the events waiting in the queue, on the tkinter thread."""
        try:
            for _ in range(BATCH):
                event = self.manager.next_event(0)
                if event is None:
                    break
                self.on_event(event)
        except sylvac_ble.StoppedError:
            return
        self.poll_id = self.root.after(POLL_MS, self.poll)

    def on_event(self, event: Event) -> None:
        match event:
            case CentralAdded(mac=mac):
                self.dongles.append(mac)
                self.dongles_changed()
            case CentralRemoved(mac=mac):
                if mac in self.dongles:
                    self.dongles.remove(mac)
                self.dongles_changed()
            case AdvertiserChanged(mac=mac, name=name, value=value):
                self.show_advertiser(mac, name, value)
            case AdvertiserRemoved(mac=mac):
                row = self.advertisers.pop(mac, None)
                if row is not None:
                    row.frame.destroy()
            # "Connected instruments" mirrors the manager's list.
            case PeripheralAdded(peripheral=mac):
                self.add_instrument(mac)
            case PeripheralRemoved(peripheral=mac):
                self.remove_instrument(mac)
            case ConnectionStepChanged(peripheral=mac, value=step):
                self.show_step(mac, step)
            case MeasurementParamsChanged(peripheral=mac, value=params):
                self.show(mac, "unit", display_name(params.unit))
                self.show(mac, "mode", display_name(params.measuring_mode))
            case NumericalValueChanged(peripheral=mac, value=value):
                self.show(mac, "value", f3(value))
            case InstrumentButtonPressed(peripheral=mac, value=text):
                self.show(mac, "data", text)
            case InstrumentResponseReceived(peripheral=mac, value=text):
                self.show(mac, "response", text)
            case LibLog() | Rejected():
                print(event)
            case _:
                pass

    def dongles_changed(self) -> None:
        """Selects the first dongle (none when the list is empty) after every change of the list."""
        self.dongle_box.configure(values=self.dongles)
        self.selected_dongle.set(self.dongles[0] if self.dongles else "")

    def show_advertiser(self, mac: str, name: str | None, value: str | None) -> None:
        row = self.advertisers.get(mac)
        if row is None:
            # Only Sylvac instruments are listed: the name contains "SY".
            if name is None or "SY" not in name:
                return
            frame = ttk.Frame(self.advertiser_list)
            frame.pack(anchor="w")
            ttk.Button(frame, text="Connect", command=lambda: self.connect_instrument(mac)).pack(
                side="left"
            )
            ttk.Label(frame, text=mac, font="TkFixedFont").pack(side="left", padx=(5, 0))
            row = AdvertiserRow(frame, ttk.Label(frame), ttk.Label(frame))
            row.name.pack(side="left", padx=(5, 0))
            row.value.pack(side="left", padx=(5, 0))
            self.advertisers[mac] = row
        if name is not None:
            row.name.configure(text=name)
        row.value.configure(text=value or "")

    def add_instrument(self, mac: str) -> None:
        """Adds the line of the instrument to "Connected instruments"."""
        if mac in self.instruments:
            return
        line = ttk.Frame(self.instrument_list)
        line.pack(anchor="w", pady=(0, 5))
        dot = tk.Canvas(line, width=12, height=12, highlightthickness=0)
        dot.create_oval(0, 0, 11, 11, fill=RED, outline="", tags="dot")
        dot.pack(side="left")
        row = InstrumentRow(line, dot)
        ttk.Label(line, text=mac, font="TkFixedFont").pack(side="left", padx=(5, 0))
        ttk.Button(line, text="Disconnect", command=lambda: self.disconnect_instrument(mac)).pack(
            side="left", padx=(5, 0)
        )
        # The line: value (three decimals), unit, measuring mode, button data, response. Before
        # the first SDS Parameters, it shows the defaults (NaN, Undefined, Normal).
        for key, text, pad, grey in (
            ("value", f3(math.nan), 10, False),
            ("unit", "Undefined", 5, False),
            ("mode", "Normal", 5, False),
            ("", "Data from the button : ", 10, False),
            ("data", "", 0, True),
            ("", "Response : ", 10, False),
            ("response", "", 0, True),
        ):
            label = ttk.Label(line, text=text, background="#eee" if grey else "")
            label.pack(side="left", padx=(pad, 0))
            if key:
                row.widgets[key] = label
        self.instruments[mac] = row

    def remove_instrument(self, mac: str) -> None:
        row = self.instruments.pop(mac, None)
        if row is not None:
            row.line.destroy()

    def show(self, mac: str, key: str, text: str) -> None:
        row = self.instruments.get(mac)
        if row is not None:
            row.widgets[key].configure(text=text)

    def show_step(self, mac: str, step: ConnectionStep) -> None:
        row = self.instruments.get(mac)
        if row is None:
            return
        if step == ConnectionStep.SUBSCRIBED:
            colour = GREEN
        elif step == ConnectionStep.NONE:
            colour = RED
        else:
            colour = ORANGE
        row.dot.itemconfigure("dot", fill=colour)

    # ─── Commands ────────────────────────────────────────────────────────────

    def start_scan(self) -> None:
        # Filter to receive only Sylvac-compatible advertisements.
        self.manager.scan.sylvac_only = True
        self.manager.scan.start()

    def stop_scan(self) -> None:
        self.manager.scan.stop()

    def clear_scan(self) -> None:
        self.manager.scan.clear()

    def connect_instrument(self, mac: str) -> None:
        """Connects the instrument through the selected dongle; nothing when none is selected."""
        central = self.selected_dongle.get()
        if not central:
            return
        self.manager.start_connect(mac, central=central)

    def disconnect_instrument(self, mac: str) -> None:
        """Disconnects the instrument and takes it off the list in one call; the line goes at
        `PeripheralRemoved`."""
        self.manager.peripheral(mac).start_disconnect()

    def send_request_to_all(self) -> None:
        """Sends the command to every instrument of the list."""
        text = self.request_text.get()
        if not text.strip():
            return
        for mac in self.instruments:
            self.manager.peripheral(mac).send_command(text)

    def close(self) -> None:
        if self.poll_id is not None:
            self.root.after_cancel(self.poll_id)
        self.manager.stop()
        self.root.destroy()


def main() -> None:
    root = tk.Tk()
    App(root)
    root.mainloop()


if __name__ == "__main__":
    main()
