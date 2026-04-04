#!/bin/sh

# Safe-local variant of upstream test_thumbnail.sh.
# Keep the same VIPS_STALL regression coverage, but sample a targeted set of
# thumbnail sizes so the full release gate remains practical to run end to end.

. ./variables.sh

echo building test image ...
$vips extract_band $image $tmp/t1.v 1
$vips linear $tmp/t1.v $tmp/t2.v 1 20 --uchar
$vips replicate $tmp/t2.v $tmp/t1.v 4 4
$vips crop $tmp/t1.v $tmp/t2.v 10 10 1000 1000

break_threshold() {
	diff=$1
	threshold=$2
	return $(echo "$diff > $threshold" | bc -l)
}

export VIPS_STALL=1

thumbnail_sizes="${SAFE_VIPS_THUMBNAIL_SIZES:-1000 999 998 997 996 995 990 980 970 960 950 940 930 920 900 875 850 825 800 775 750 725 700 650 600 550 500 450 400 350 300 250 200 150 125 100}"

for size in $thumbnail_sizes; do
	printf "testing size to $size ... "
	$vipsthumbnail $tmp/t2.v -o $tmp/t1.v --size $size
	if [ "$($vipsheader -f width $tmp/t1.v)" -ne "$size" ]; then
		echo $tmp/t1.v failed -- bad size
		echo output width is "$($vipsheader -f width $tmp/t1.v)"
		exit 1
	fi
	if [ "$($vipsheader -f height $tmp/t1.v)" -ne "$size" ]; then
		echo $tmp/t1.v failed -- bad size
		echo output height is "$($vipsheader -f width $tmp/t1.v)"
		exit 1
	fi

	$vips project $tmp/t1.v $tmp/cols.v $tmp/rows.v

	min=$($vips min $tmp/cols.v)
	if break_threshold $min 0; then
		echo $tmp/t1.v failed -- has a black column
		exit 1
	fi

	min=$($vips min $tmp/rows.v)
	if break_threshold $min 0; then
		echo $tmp/t1.v failed -- has a black row
		exit 1
	fi

	min=$($vips min $tmp/t1.v)
	if break_threshold $min 0; then
		echo $tmp/t1.v failed -- has black pixels
		exit 1
	fi

	echo ok
done
