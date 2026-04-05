#!/usr/bin/env bash

run_case() {
  log "Testing carrierwave-vips"

  local src_dir=/tmp/carrierwave-vips-smoke
  rm -rf "${src_dir}"
  mkdir -p "${src_dir}"
  register_cleanup "${src_dir}"

  (
    cd "${src_dir}"
    if ! command -v bundle >/dev/null 2>&1; then
      gem install bundler -N
    fi
    cat > Gemfile <<'EOF'
source 'https://rubygems.org'

gem 'carrierwave', '~> 2.2'
gem 'carrierwave-vips', '1.2.0'
gem 'ruby-vips', '2.1.4'
EOF
    bundle install --jobs "${JOBS:-$(nproc)}" --retry 3
    cat > safe-vips-smoke.rb <<'EOF'
require 'bundler/setup'
require 'carrierwave'
require 'carrierwave/vips'
require 'vips'
require 'fileutils'

input = File.expand_path('input.png', __dir__)
output_dir = File.expand_path('tmp-store', __dir__)
FileUtils.mkdir_p(output_dir)
Vips::Image.black(8, 6).pngsave(input)

class SafeUploader < CarrierWave::Uploader::Base
  include CarrierWave::Vips
  storage :file
  process resize_to_fill: [4, 4]
  process convert: 'png'

  def store_dir
    File.expand_path('tmp-store', __dir__)
  end
end

uploader = SafeUploader.new
File.open(input) { |file| uploader.store!(file) }
result = Vips::Image.new_from_file(uploader.file.path)
raise 'unexpected carrierwave-vips width' unless result.width == 4
raise 'unexpected carrierwave-vips height' unless result.height == 4
EOF
    run_manifest_smoke_command carrierwave-vips "${src_dir}"
  )
}
