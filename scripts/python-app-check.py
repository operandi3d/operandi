# The standard library a script can count on, and help(operandi) (pydoc
# reads every function's text). Run inside Operandi: by the operandi-python
# tests, and against a built app by scripts/check-bundled-python.sh.
import importlib
names = '''inspect dis pydoc json re math datetime decimal fractions statistics random
collections itertools functools pathlib os sys struct array zlib hashlib ssl sqlite3 csv
unicodedata typing dataclasses enum asyncio threading subprocess tempfile textwrap copy
pickle io time _opcode'''.split()
missing = []
for n in names:
    try:
        importlib.import_module(n)
    except Exception as err:
        missing.append(f'{n}: {err}')
import pydoc, operandi
text = pydoc.render_doc(operandi, renderer=pydoc.plaintext)
print('missing', missing)
print('help', 'mesh_get' in text and 'undo_group' in text)
