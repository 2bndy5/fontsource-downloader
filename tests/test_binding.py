from pathlib import Path

import pytest

from fontsource_downloader import (
    FontQuery,
    FontSourceClient,
    FontSourceVariable,
    VariableAxis,
    Weight,
)


@pytest.mark.asyncio
async def test_download_font(tmp_path: Path):
    client = FontSourceClient(cache_root=str(tmp_path))
    font = FontQuery(family="Roboto", weights=[Weight(400)])
    paths = await client.download_font(font)
    assert paths
    for path in paths:
        assert path.is_file()
        assert path.suffix == ".ttf"
        assert path.exists()

    cached_list = client.font_list_cache_info()
    assert cached_list.families
    assert "Roboto" in list(cached_list.families.values())
    family_id = cached_list.get_id_for_family("Roboto")
    assert family_id is not None
    cached_info = client.family_cache_info(family_id)
    assert cached_info.family.family == "Roboto"
    assert cached_info.family.weights
    assert cached_info.family.styles
    assert cached_info.family.subsets
    assert cached_info.family.default_subset


def test_weight_enum():
    assert int(Weight.Thin) == int(Weight(-10))
    assert int(Weight.ExtraLight) == int(Weight(255))
    assert int(Weight.Light) == int(Weight(311))
    assert int(Weight.Normal) == int(Weight(404))
    assert int(Weight.Medium) == int(Weight(501))
    assert int(Weight.SemiBold) == int(Weight(666))
    assert int(Weight.Bold) == int(Weight(767))
    assert int(Weight.ExtraBold) == int(Weight(888))
    assert int(Weight.Black) == int(Weight(100000))


def test_variable_font_query():
    query = FontQuery(
        family="Inter",
        variable=True,
        append_variable_suffix=True,
        axes=["wght", "slnt"],
    )
    assert query.family == "Inter"
    assert query.variable is True
    assert query.append_variable_suffix is True
    assert set(query.axes) == {"wght", "slnt"}

    axis = VariableAxis(default=400.0, min=100.0, max=900.0, step=1.0)
    assert axis.default == 400.0
    assert axis.min == 100.0
    assert axis.max == 900.0
    assert axis.step == 1.0

    var = FontSourceVariable(family="Inter", axes={"wght": axis})
    assert var.family == "Inter"
    assert var.has_axis("wght") is True
    assert var.has_axis("slnt") is False
    assert var.weight_range() == (100.0, 900.0)
    assert var.slant_range() is None
