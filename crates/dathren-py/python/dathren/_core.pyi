from typing import Optional

__version__: str

def datasets_json() -> str: ...
def info_json(dataset: str) -> str: ...
def fetch_json(
    dataset: str,
    geo: Optional[str] = None,
    measures: list[str] = ...,
    since: Optional[int] = None,
) -> str: ...
