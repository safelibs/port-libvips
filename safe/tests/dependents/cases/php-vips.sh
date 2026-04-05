#!/usr/bin/env bash

run_case() {
  log "Testing php-vips"

  local src_dir=/tmp/php-vips-src
  clone_git_ref php-vips "${src_dir}"
  register_cleanup "${src_dir}"

  (
    cd "${src_dir}"
    composer install --no-interaction --no-progress
    cat > safe-vips-smoke.php <<'PHP'
<?php
require __DIR__ . '/vendor/autoload.php';

use Jcupitt\Vips;

$input = __DIR__ . '/safe-vips-input.pgm';
$output = __DIR__ . '/safe-vips-output.png';
file_put_contents($input, "P5\n8 6\n255\n" . str_repeat(chr(0), 8 * 6));

$image = Vips\Image::newFromFile($input);
if ($image->width !== 8 || $image->height !== 6) {
    throw new RuntimeException('unexpected php-vips image dimensions');
}
$image = $image->invert();
$image->writeToFile($output);
if (!file_exists($output) || filesize($output) === 0) {
    throw new RuntimeException('php-vips did not write an output file');
}
PHP
    run_manifest_smoke_command php-vips "${src_dir}"
  )
}
