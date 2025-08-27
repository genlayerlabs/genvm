# { "Depends": "py-genlayer:test" }

# 6 color rainbow
im_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x80\x00\x00\x00O\x04\x03\x00\x00\x00\xd1\xae\xd0\x99\x00\x00\x00'PLTE@@\xff\xff\x81\x00\x00y?\xff\xff\x00\xa0\x00\xc0\xf0\x00\x00R4\xf3\x00oE\xfch\x00\xff\xd4\x00\x80\xbc \x16f\x80\xffx\x00\xeeM\xf2\xd3\x00\x00\x00VIDATX\xc3\xed\xccA\x11\x800\x10\x04\xc1\xb5\x10\x0b\xb1\x80\x05,`!\x16\xb0\x80\x05,`!\x16\x10\xc5\xe3$\xcc'EM\x0b\xe8\x1cP\x0c\x0c\x0c*\xb8\xa04\xc8\xc0`\x91\xe0\x85rC\xd9 \x03\x83\xbf\x04\x0f\x94\x13J\x87\x0c\x0c\x16\t&\x14Ie@\xd9!\x03\x03\x83\xf2\x01\xb1\xdb\"}Y/;:\x00\x00\x00\x00IEND\xaeB`\x82"

import sys
from genlayer import *
import io
import re


class Contract(gl.Contract):
	@gl.public.view
	def main(self):
		def run():
			import PIL.Image as Image
			from PIL import ImageOps

			im = Image.open(io.BytesIO(im_data))

			# Invert the colors
			inverted_im = ImageOps.invert(im.convert('RGB'))

			# Convert back to bytes
			inverted_buffer = io.BytesIO()
			inverted_im.save(inverted_buffer, format='PNG')
			inverted_buffer.flush()
			inverted_im_data = inverted_buffer.getvalue()

			res = gl.nondet.exec_prompt(
				'how are these images different? Which filter can convert one to another? Choose from: color inversion, mirroring, blur, contrast, brightness. Be as concise as possible, respond with a single sentence',
				images=[im_data, inverted_im_data],
			)
			print(res, file=sys.stderr)
			res = res.strip().lower()
			res = re.sub(r'[^a-z]', '', res)
			for could in ['invert', 'inversion', 'color shift', 'hue shift']:
				if could in res:
					return True
			return False

		res = gl.eq_principle.strict_eq(run)
		print(res)
