import json, os, subprocess, tempfile

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets', 'client', 'textures', 'gui')
PX = 96

ICONS = {
    'category/combat': '''
        <path d="M14.5 17.5 4 7V4h3l10.5 10.5"/>
        <path d="M13 19l6-6"/><path d="M16.5 16.5 20 20"/><path d="M19 21l2-2"/>
        <path d="M14.5 6.5 17 4h3v3l-2.5 2.5"/>
        <path d="M5 14l4 4"/><path d="M7 17l-3 3"/><path d="M3 19l2 2"/>
    ''',
    'category/movement': '''
        <path d="M3 8h9.5a2.75 2.75 0 1 0-2.75-2.75"/>
        <path d="M3 12h14.5a2.75 2.75 0 1 1-2.75 2.75"/>
        <path d="M3 16h7"/>
    ''',
    'category/render': '''
        <path d="M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7S2.5 12 2.5 12z"/>
        <circle cx="12" cy="12" r="3"/>
    ''',
    'category/player': '''
        <circle cx="12" cy="8" r="4"/>
        <path d="M4.5 20.5v-.5a6 6 0 0 1 6-6h3a6 6 0 0 1 6 6v.5"/>
    ''',
    'category/util': '''
        <path d="M14.5 6.5a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.4-3.4a6 6 0 0 1-7.8 7.8l-6.9 6.9a2.1 2.1 0 0 1-3-3l6.9-6.9a6 6 0 0 1 7.8-7.8z"/>
    ''',
    'category/configs': '''
        <path d="M4 6h8"/><path d="M18 6h2"/><circle cx="15" cy="6" r="2.5"/>
        <path d="M4 12h2"/><path d="M12 12h8"/><circle cx="9" cy="12" r="2.5"/>
        <path d="M4 18h10"/><circle cx="17" cy="18" r="2.5"/>
    ''',
    'category/themes': '''
        <path d="M12 21.5a9.5 9.5 0 1 1 9.5-9.5c0 2.6-2.1 3.8-4.3 3.8h-2.1a1.9 1.9 0 0 0-1.4 3.1 1.6 1.6 0 0 1-1.7 2.6z"/>
        <circle cx="7.5" cy="11" r="1.2" fill="#fff" stroke="none"/>
        <circle cx="10.5" cy="7" r="1.2" fill="#fff" stroke="none"/>
        <circle cx="15.5" cy="7.5" r="1.2" fill="#fff" stroke="none"/>
    ''',
    'logout': '''
        <path d="M9.5 20.5H6a2 2 0 0 1-2-2v-13a2 2 0 0 1 2-2h3.5"/>
        <path d="M15.5 16.5 20 12l-4.5-4.5"/>
        <path d="M20 12H9.5"/>
    ''',
}

SVG = '''<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 24 24"
fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{body}</svg>'''

for name, body in ICONS.items():
    out = os.path.join(OUT, name + '.png')
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with tempfile.NamedTemporaryFile('w', suffix='.svg', delete=False) as f:
        f.write(SVG.format(px=PX, body=body))
        svg = f.name
    subprocess.run(['rsvg-convert', '-w', str(PX), '-h', str(PX), svg, '-o', out], check=True)
    os.unlink(svg)
    with open(out + '.mcmeta', 'w') as f:
        json.dump({'texture': {'blur': True, 'clamp': True}}, f)
    print('wrote', out)
