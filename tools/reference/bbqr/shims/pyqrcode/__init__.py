"""The one name BBQr's `utils.version_to_chars` takes from pyqrcode:
`pyqrcode.tables.data_capacity[version]["L"][2]`, the alphanumeric
characters a code of that version holds at level L. Written for the
cross-check (see ../../README.md); pyqrcode itself is not vendored."""

from . import tables  # noqa: F401
