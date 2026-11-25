#!/usr/bin/env python3
import json
import os
import struct
import sys
import zlib

import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

NAME = "org.freedesktop.Notifications"
PATH = "/org/freedesktop/Notifications"
OUT = sys.argv[1] if len(sys.argv) > 1 else "notifications.jsonl"
CLICK = os.environ.get("FAKE_NOTIFYD_CLICK") == "1"


def png(path, w, h, stride, data):
    raw = b"".join(b"\x00" + bytes(data[y * stride:y * stride + w * 4]) for y in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


class Server(dbus.service.Object):
    def __init__(self, bus):
        super().__init__(dbus.service.BusName(NAME, bus), PATH)
        self.next = 0

    def log(self, entry):
        with open(OUT, "a") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    @dbus.service.method(NAME, in_signature="", out_signature="as")
    def GetCapabilities(self):
        return ["actions", "body", "body-markup", "icon-static", "persistence"]

    @dbus.service.method(NAME, in_signature="", out_signature="ssss")
    def GetServerInformation(self):
        return ("phoenix-fake-notifyd", "phoenix", "1", "1.2")

    @dbus.service.method(NAME, in_signature="susssasa{sv}i", out_signature="u")
    def Notify(self, app, replaces, icon, summary, body, actions, hints, timeout):
        nid = int(replaces) if replaces else self.next + 1
        self.next = max(self.next, nid)
        entry = {"call": "Notify", "id": nid, "app_name": str(app), "replaces_id": int(replaces), "app_icon": str(icon), "summary": str(summary), "body": str(body), "actions": [str(a) for a in actions], "expire_timeout": int(timeout), "hints": {}}
        for k, v in hints.items():
            if k == "image-data":
                w, h, stride, alpha, bps, ch, data = v
                entry["hints"][k] = {"width": int(w), "height": int(h), "rowstride": int(stride), "has_alpha": bool(alpha), "bits": int(bps), "channels": int(ch), "bytes": len(data)}
                png(OUT + f".{nid}.png", int(w), int(h), int(stride), bytes(data))
            else:
                entry["hints"][k] = bool(v) if isinstance(v, dbus.Boolean) else v if isinstance(v, (int, float)) else str(v)
        self.log(entry)
        if CLICK:
            GLib.timeout_add(500, self.click, nid)
        return dbus.UInt32(nid)

    @dbus.service.method(NAME, in_signature="u", out_signature="")
    def CloseNotification(self, nid):
        self.log({"call": "CloseNotification", "id": int(nid)})
        self.NotificationClosed(nid, 3)

    def click(self, nid):
        self.ActivationToken(nid, f"fake-token-{nid}")
        self.ActionInvoked(nid, "default")
        self.log({"call": "emitted", "id": int(nid), "signals": ["ActivationToken", "ActionInvoked"]})
        return False

    @dbus.service.signal(NAME, signature="us")
    def ActionInvoked(self, nid, key):
        pass

    @dbus.service.signal(NAME, signature="us")
    def ActivationToken(self, nid, token):
        pass

    @dbus.service.signal(NAME, signature="uu")
    def NotificationClosed(self, nid, reason):
        pass


if __name__ == "__main__":
    if "/run/user/" in os.environ.get("DBUS_SESSION_BUS_ADDRESS", "/run/user/"):
        sys.exit("refusing to own org.freedesktop.Notifications on the desktop session bus")
    DBusGMainLoop(set_as_default=True)
    Server(dbus.SessionBus())
    GLib.MainLoop().run()
