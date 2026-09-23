"""Generate RepoTower's original circuit mark and desktop icon assets.

Uses only Pillow already installed in the build environment. No downloaded art,
fonts or image services. Run from any directory; outputs stay in desktop/assets.
"""
from pathlib import Path
from io import BytesIO
import struct
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
DESTINATION = ROOT / "desktop" / "assets"
DESTINATION.mkdir(parents=True, exist_ok=True)
SCALE = 4

canvas = Image.new("RGBA", (512 * SCALE, 512 * SCALE), (0, 0, 0, 0))
draw = ImageDraw.Draw(canvas)

def points(values):
    return [(round(x * SCALE), round(y * SCALE)) for x, y in values]

draw.rounded_rectangle((16*SCALE, 16*SCALE, 496*SCALE, 496*SCALE), radius=108*SCALE,
                       fill="#151b20", outline="#35414a", width=3*SCALE)
svg = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">',
       '<rect x="16" y="16" width="480" height="480" rx="108" fill="#151b20" stroke="#35414a" stroke-width="3"/>']

def route(vertices, color, width):
    draw.line(points(vertices), fill=color, width=width*SCALE, joint="curve")
    for x, y in vertices:
        radius = width * SCALE / 2
        draw.ellipse((x*SCALE-radius, y*SCALE-radius, x*SCALE+radius, y*SCALE+radius), fill=color)
    svg.append('<polyline points="' + ' '.join(f'{x},{y}' for x, y in vertices)
               + f'" fill="none" stroke="{color}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"/>')

route([(166, 166), (346, 166)], "#445d6c", 18)
route([(166, 166), (166, 346)], "#445d6c", 18)
route([(166, 346), (284, 346), (346, 284), (346, 166)], "#445d6c", 18)

def pad(cx, cy, color, center):
    # The large rounded pads read clearly at native 16px and 24px icon sizes.
    draw.rounded_rectangle(((cx-54)*SCALE, (cy-54)*SCALE, (cx+54)*SCALE, (cy+54)*SCALE),
                           radius=27*SCALE, fill=color)
    draw.rounded_rectangle(((cx-21)*SCALE, (cy-21)*SCALE, (cx+21)*SCALE, (cy+21)*SCALE),
                           radius=8*SCALE, fill=center)
    svg.append(f'<rect x="{cx-54}" y="{cy-54}" width="108" height="108" rx="27" fill="{color}"/>')
    svg.append(f'<rect x="{cx-21}" y="{cy-21}" width="42" height="42" rx="8" fill="{center}"/>')

pad(166, 166, "#6ce5bf", "#1d6355")
pad(346, 166, "#74b5fa", "#284f7e")
pad(166, 346, "#b5a0ff", "#514174")
svg.append('</svg>')
(DESTINATION / "repotower.svg").write_text('\n'.join(svg) + '\n', encoding="utf-8")

canvas = canvas.resize((512, 512), getattr(Image, "Resampling", Image).LANCZOS)
canvas.save(DESTINATION / "repotower.png")
canvas.save(DESTINATION / "repotower.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
# The ICNS container supports PNG payloads directly; writing the tiny container
# ourselves also supports older Pillow versions without the ICNS save plugin.
icns_chunks = []
for kind, size in [(b"ic07",128), (b"ic08",256), (b"ic09",512), (b"ic10",1024)]:
    payload = BytesIO()
    canvas.resize((size,size), getattr(Image, "Resampling", Image).LANCZOS).save(payload, format="PNG")
    data = payload.getvalue()
    icns_chunks.append(kind + struct.pack(">I", len(data)+8) + data)
icns_body = b"".join(icns_chunks)
(DESTINATION / "repotower.icns").write_bytes(b"icns" + struct.pack(">I", len(icns_body)+8) + icns_body)
print(f"Created RepoTower SVG, PNG, ICO and ICNS in {DESTINATION}")
