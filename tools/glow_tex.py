import json, os
import numpy as np
from PIL import Image

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets', 'client', 'textures', 'gui', 'glow')
N = 128
C = N / 2
LEVELS = 16
os.makedirs(OUT, exist_ok=True)
SS = 4
xs = (np.arange(N * SS) + 0.5) / SS
X, Y = np.meshgrid(xs, xs)
rho = np.sqrt((X - C) ** 2 + (Y - C) ** 2) / C
for i in range(LEVELS):
    a = i / LEVELS
    d = (rho - a) / (1 - a)
    alpha = np.where(d < 0, 0.0, np.clip(1 - d, 0, 1) ** 2)
    alpha = alpha.reshape(N, SS, N, SS).mean(axis=(1, 3))
    img = np.zeros((N, N, 4), np.uint8)
    img[..., :3] = 255
    img[..., 3] = np.round(alpha * 255).astype(np.uint8)
    p = os.path.join(OUT, f'g{i}.png')
    Image.fromarray(img, 'RGBA').save(p)
    with open(p + '.mcmeta', 'w') as f:
        json.dump({'texture': {'blur': True, 'clamp': True}}, f)
print('glow textures written')
