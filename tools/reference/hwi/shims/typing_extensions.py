"""Written here, not upstream: HWI's `_serialize.py` imports `Protocol`
from the `typing_extensions` package, for type hints only. The standard
library has had it since Python 3.8."""

from typing import Protocol  # noqa: F401
