group "default" {
  targets = ["image"]
}

target "image" {
  name = replace(image, ".", "-")
  matrix = {
    image = ["ubuntu-24.04"]
  }
  context = "images/${image}"
  platforms = ["linux/amd64"]
  pull = true
  tags = ["ghcr.io/threeal/homerunner:${image}"]
}
