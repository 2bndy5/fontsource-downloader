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
    print(f"Downloaded font to: {path}")


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
