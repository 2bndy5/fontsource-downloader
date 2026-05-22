from dataclasses import dataclass
from enum import IntEnum
from pathlib import Path
from typing import NamedTuple

class FontSourceClient:
    def __init__(self, cache_root: Path | None = None) -> None: ...
    async def download_font(self, font: FontQuery) -> Path: ...
    def families_cache_path(self) -> Path: ...
    def font_list_cache_info(self) -> FontListCacheInfo: ...
    def family_cache_info(self, family: str) -> FamilyCacheInfo: ...

class Weight(IntEnum):
    Thin = 100
    ExtraLight = 200
    Light = 300
    Normal = 400
    Medium = 500
    SemiBold = 600
    Bold = 700
    ExtraBold = 800
    Black = 900

    def __init__(self, value: int) -> None: ...
    def __int__(self) -> int: ...

class FontQuery(NamedTuple):
    family: str = ...
    style: str | None = ...
    weight: Weight | None = ...
    subset: str | None = ...

@dataclass(frozen=True)
class FontListCacheInfo:
    expiration: int
    families: dict[str, str]

    def get_id_for_family(self, family_name: str) -> str | None: ...

@dataclass(frozen=True)
class FamilyCacheInfo:
    expiration: int
    family: FontSourceFamily

@dataclass(frozen=True)
class FontSourceFamily:
    id: str
    family: str
    subsets: list[str]
    weights: list[int]
    styles: list[str]
    default_subset: str

    def variant_ttf_url(self, weight: int, style: str, subset: str) -> str | None: ...
