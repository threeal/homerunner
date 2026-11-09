group "default" {
  targets = ["image"]
}

target "image" {
  name = replace(image, ".", "-")
  matrix = {
    image = ["ubuntu-24.04"]
  }
  context = "images/${image}"
  args = {
    SOURCE_DATE_EPOCH = 0
  }
  platforms = ["linux/amd64"]
  pull = true
  attest = ["type=provenance,disabled=true"]
  tags = ["ghcr.io/threeal/homerunner:${image}"]
}
