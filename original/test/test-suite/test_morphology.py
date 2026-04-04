# vim: set fileencoding=utf-8 :
import pytest

import pyvips


class TestMorphology:
    def test_countlines(self):
        im = pyvips.Image.black(100, 100)
        im = im.draw_line(255, 0, 50, 100, 50)
        n_lines = im.countlines(pyvips.Direction.HORIZONTAL)
        assert n_lines == 1

    def test_labelregions(self):
        im = pyvips.Image.black(100, 100)
        im = im.draw_circle(255, 50, 50, 25, fill=True)
        mask, opts = im.labelregions(segments=True)

        assert opts['segments'] == 3
        assert mask.max() == 2

    def test_fill_nearest(self):
        im = pyvips.Image.black(7, 1, bands=3)
        im = im.draw_rect([10, 20, 30], 0, 0, 1, 1, fill=True)
        im = im.draw_rect([100, 110, 120], 6, 0, 1, 1, fill=True)
        filled, opts = im.fill_nearest(distance=True)

        distance = opts['distance']

        assert filled.width == im.width
        assert filled.height == im.height
        assert filled.bands == im.bands
        assert distance.width == im.width
        assert distance.height == im.height
        assert distance.bands == 1
        assert distance.format == pyvips.BandFormat.FLOAT

        assert filled(0, 0) == [10.0, 20.0, 30.0]
        assert filled(2, 0) == [10.0, 20.0, 30.0]
        assert filled(5, 0) == [100.0, 110.0, 120.0]
        assert distance(0, 0)[0] == pytest.approx(0.0)
        assert distance(2, 0)[0] == pytest.approx(2.0)
        assert distance(5, 0)[0] == pytest.approx(1.0)

    def test_erode(self):
        im = pyvips.Image.black(100, 100)
        im = im.draw_circle(255, 50, 50, 25, fill=True)
        im2 = im.erode([[128, 255, 128],
                        [255, 255, 255],
                        [128, 255, 128]])
        assert im.width == im2.width
        assert im.height == im2.height
        assert im.bands == im2.bands
        assert im.avg() > im2.avg()

    def test_dilate(self):
        im = pyvips.Image.black(100, 100)
        im = im.draw_circle(255, 50, 50, 25, fill=True)
        im2 = im.dilate([[128, 255, 128],
                         [255, 255, 255],
                         [128, 255, 128]])
        assert im.width == im2.width
        assert im.height == im2.height
        assert im.bands == im2.bands
        assert im2.avg() > im.avg()

    def test_rank(self):
        im = pyvips.Image.black(100, 100)
        im = im.draw_circle(255, 50, 50, 25, fill=True)
        im2 = im.rank(3, 3, 8)
        assert im.width == im2.width
        assert im.height == im2.height
        assert im.bands == im2.bands
        assert im2.avg() > im.avg()


if __name__ == '__main__':
    pytest.main()
