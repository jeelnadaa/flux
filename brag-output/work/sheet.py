from PIL import Image
from pathlib import Path
fs = sorted(Path('stills').glob('*.png'), key=lambda f: float(f.stem))
ims = [Image.open(f).convert('RGB').resize((640, 360)) for f in fs]
for k in range(0, len(ims), 6):
    sheet = Image.new('RGB', (1920, 720), 'white')
    for i, im in enumerate(ims[k:k + 6]):
        sheet.paste(im, ((i % 3) * 640, (i // 3) * 360))
    sheet.save(f'sheet{k // 6}.png')
print([f.stem for f in fs])
