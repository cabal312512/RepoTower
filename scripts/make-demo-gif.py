"""Optional slideshow of real 2D desktop captures; requires existing Pillow."""
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parent.parent
sources = [root / 'docs' / 'images' / name for name in
           ['circuit-dark.png', 'circuit-light.png', 'circuit-impact.png']]
if not all(source.exists() for source in sources):
    raise SystemExit('First run node scripts/capture-native.mjs')
images = []
for source in sources:
    with Image.open(source) as original:
        width = min(980, original.width)
        frame = original.convert('RGB').resize((width, round(original.height * width / original.width)))
        images.append(frame.quantize(colors=160))
durations = [2000, 1800, 3000]
images[0].save(root / 'docs' / 'images' / 'circuit.gif', save_all=True,
               append_images=images[1:], duration=durations, loop=0, optimize=True)
print(root / 'docs' / 'images' / 'circuit.gif')
