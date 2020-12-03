import colorsys
import time


spacing = 360.0 / 16.0
hue = 0

# blinkt.set_clear_on_exit()
# blinkt.set_brightness(0.1)

while True:
	hue = int(time.time() * 100) % 360
	for x in range(8):
		offset = x * spacing
		h = ((hue + offset) % 360) / 360.0
		r, g, b = [int(c * 255) for c in colorsys.hsv_to_rgb(h, 1.0, 1.0)]
		print(hue, h, r, g, b, colorsys.hsv_to_rgb(h, 1.0, 1.0))
		# blinkt.set_pixel(x, r, g, b)

	# blinkt.show()
	time.sleep(0.001)