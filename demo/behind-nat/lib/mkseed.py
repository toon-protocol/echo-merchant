#!/usr/bin/env python3
"""Build a cloud-init NoCloud seed ISO (volume label cidata) without root.

  uv run --with pycdlib python mkseed.py OUT.iso user-data meta-data
"""
import io, sys
import pycdlib

out, user_data, meta_data = sys.argv[1:4]
iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, joliet=3, rock_ridge="1.09", vol_ident="cidata")
for path, name in ((user_data, "user-data"), (meta_data, "meta-data")):
    data = open(path, "rb").read()
    iso.add_fp(io.BytesIO(data), len(data), "/" + name.upper().replace("-", "_") + ".;1",
               rr_name=name, joliet_path="/" + name)
iso.write(out)
iso.close()
