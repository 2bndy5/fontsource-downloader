from pathlib import Path
import pytest

from fontsource_downloader import FontQuery, FontSourceClient, Weight


@pytest.mark.asyncio
async def test_download_font(tmp_path: Path):
    client = FontSourceClient(cache_root=str(tmp_path))
    font = FontQuery(family="Roboto", weight=Weight(400))
    path = await client.download_font(font)
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
    assert cached_info.family.variant_ttf_url(400, "normal", "latin") is not None
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
