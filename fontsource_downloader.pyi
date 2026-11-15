from dataclasses import dataclass
from enum import Enum, IntEnum
from pathlib import Path
from typing import NamedTuple

class FontSourceClient:
    def __init__(self, cache_root: Path | None = None) -> None: ...
    async def download_font(self, font: FontQuery) -> Path: ...
    def families_cache_path(self) -> Path: ...
    def font_list_cache_info(self) -> FontListCacheInfo: ...
    def family_cache_info(self, family: str) -> FamilyCacheInfo: ...
    def variable_cache_info(self, id: str) -> VariableCacheInfo: ...
    async def variable_metadata(self, family: str) -> FontSourceVariable: ...

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

class FontFileType(Enum):
    Woff2 = ...
    Woff = ...
    Ttf = ...

    def __str__(self) -> str: ...

class FontQuery(NamedTuple):
    family: str = ...
    styles: list[str] | None = ...
    weights: list[Weight] | None = ...
    subsets: list[str] | None = ...
    file_types: list[FontFileType] | None = ...
    variable: bool = ...
    append_variable_suffix: bool = ...
    axes: list[str] | None = ...

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
class VariableCacheInfo:
    expiration: int
    variable: FontSourceVariable

@dataclass(frozen=True)
class FontSourceFamily:
    id: str
    family: str
    subsets: list[str]
    weights: list[int]
    styles: list[str]
    default_subset: str
    variable: bool

    def is_variable(self) -> bool: ...

@dataclass(frozen=True)
class VariableAxis:
    default: float
    min: float
    max: float
    step: float

@dataclass(frozen=True)
class FontSourceVariable:
    family: str
    axes: dict[str, VariableAxis]

    def has_axis(self, tag: str) -> bool: ...
    def get_axis(self, tag: str) -> VariableAxis | None: ...
    def weight_range(self) -> tuple[float, float] | None: ...
    def slant_range(self) -> tuple[float, float] | None: ...
