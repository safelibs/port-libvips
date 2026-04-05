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
		VImage image = VImage::new_from_file(argv[1]);
		double avg = image.avg();
		VImage pixel = image.crop(0, 0, 1, 1);
		std::vector<double> values = pixel(0, 0);

		if (values.empty()) {
			std::cerr << "empty pixel sample" << std::endl;
			vips_shutdown();
			return 1;
		}

		std::cout << image.width() << "x" << image.height()
			  << " avg=" << avg
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
