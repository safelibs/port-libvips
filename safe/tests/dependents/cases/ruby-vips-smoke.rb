#!/usr/bin/env ruby

require "tmpdir"

$LOAD_PATH.unshift(File.expand_path("lib", Dir.pwd))
require "vips"

def assert_close(actual, expected, epsilon, label)
  return if (actual - expected).abs <= epsilon

  raise "#{label}: expected #{expected}, got #{actual}"
end

image = Vips::Image.black(8, 6).linear(1, 128)
assert_close(image.avg, 128.0, 0.001, "generated image avg")

Dir.mktmpdir("ruby-vips-smoke") do |dir|
  path = File.join(dir, "smoke.png")
  image.write_to_file(path)

  loaded = Vips::Image.new_from_file(path)
  raise "loaded image width mismatch: #{loaded.width}" unless loaded.width == 8
  raise "loaded image height mismatch: #{loaded.height}" unless loaded.height == 6
  raise "unexpected loader metadata: #{loaded.get('vips-loader').inspect}" unless loaded.get("vips-loader") == "pngload"

  assert_close(loaded.avg, 128.0, 0.001, "loaded image avg")
  shifted = loaded.linear(1, 10)
  assert_close(shifted.avg, 138.0, 0.001, "shifted image avg")
end

puts "ruby-vips smoke passed"
