#include <iostream>
#include <vector>
#include <vips/vips8>

using namespace vips;

int main(int argc, char **argv)
{
	if (VIPS_INIT(argv[0]))
		vips_error_exit(nullptr);

	if (argc != 2)
		vips_error_exit("usage: %s input-file", argv[0]);

	try {
		VSource source = VSource::new_from_file(argv[1]);
		VImage image = VImage::new_from_source(source, "");
		VInterpolate interpolate = VInterpolate::new_from_name("nearest");
		VRegion region = VRegion::new_from_image(image);
		VTarget target = VTarget::new_to_memory();
		double avg = image.avg();
		std::vector<double> values = image.getpoint(0, 0);

		region.prepare(0, 0, 1, 1);
		image.write_to_target(".png", target);

		if (values.empty() || interpolate.get_interpolate() == nullptr ||
		    target.get_target() == nullptr) {
			std::cerr << "incomplete C++ wrapper sample" << std::endl;
			vips_shutdown();
			return 1;
		}

		std::cout << image.width() << "x" << image.height()
			  << " avg=" << avg
			  << " first-byte=" << static_cast<int>(region(0, 0))
			  << " first-band=" << values[0]
			  << std::endl;
	} catch (const VError &error) {
		std::cerr << error.what() << std::endl;
		vips_shutdown();
		return 1;
	}

	vips_shutdown();
	return 0;
}
