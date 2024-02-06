import machine
from machine import Pin
import utime

from ssd1306 import SSD1306_I2C

class ProgressBar:
    def __init__(self, x, y, length, width):
        self.x = x
        self.y = y
        self.length = length
        self.width = width

    def draw(self, ratio, screen):
        fill = int(self.length * ratio)
        screen.fill_rect(self.x, self.y, fill, self.width, 1)

oled_reset = Pin(4, Pin.OUT)
start = Pin(5, Pin.IN)
led = Pin(25, Pin.OUT)

oled_bus = machine.I2C(0, sda=Pin(0), scl=Pin(1), freq=400000)

oled_reset.value(1)
oled_reset.value(0)
utime.sleep(0.01)
oled_reset.value(1)

oled = SSD1306_I2C(128, 64, oled_bus)

pb1 = ProgressBar(10, 35, 108, 10)

index = 0
while True:
    while start.value() == 0:
        continue
    #while start.value() == 1:
        #continue
    oled.fill(0)
    progress = (index % 100) * (1.0 / 100.0)
    pb1.draw(progress, oled)
    oled.show()
    index += 1
