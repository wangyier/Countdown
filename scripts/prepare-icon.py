"""Prepare icons from the supplied image without redrawing or removing dark details."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
original = Image.open(ROOT / "icon-1.png").convert("RGBA")
bounds = original.getbbox()
if bounds is None:
    raise ValueError("The supplied icon is fully transparent")
artwork = original.crop(bounds)
# Preserve the original pixels and alpha; only remove empty margins and center.
side = max(512, *artwork.size)
source = Image.new("RGBA", (side, side))
source.paste(artwork, ((side - artwork.width) // 2, (side - artwork.height) // 2))
source.save(ROOT / "src/assets/app-icon.png")
tray = ROOT / "src-tauri/icons/tray"
tray.mkdir(parents=True, exist_ok=True)
for size in (16, 20, 24, 32):
    source.resize((size, size), Image.Resampling.LANCZOS).save(tray / f"{size}x{size}.png")
print(f"Preserved original RGBA crop {bounds} on {side}x{side} transparent canvas")
