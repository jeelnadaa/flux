# Test stand-in for Resemble's Perth watermarker: the voice engine's install check needs it to import.


class PerthImplicitWatermarker:
    def apply_watermark(self, wav, sample_rate):
        return wav
