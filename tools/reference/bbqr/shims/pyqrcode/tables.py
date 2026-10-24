"""ISO/IEC 18004's alphanumeric capacity at level L, versions 1 to 40:
the only part of pyqrcode's `tables.data_capacity` BBQr reads (mode 2
is alphanumeric in pyqrcode's numbering). The cross-check's `bbqr` area
checks every number against Nayuki's `qrcodegen.py` before it runs."""

ALNUM_L = [25, 47, 77, 114, 154, 195, 224, 279, 335, 395, 468, 535, 619, 667, 758, 854,
           938, 1046, 1153, 1249, 1352, 1460, 1588, 1704, 1853, 1990, 2132, 2223, 2369,
           2520, 2677, 2840, 3009, 3183, 3351, 3537, 3729, 3927, 4087, 4296]

data_capacity = {v: {"L": {2: ALNUM_L[v - 1]}} for v in range(1, 41)}
