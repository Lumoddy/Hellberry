package packet

type PinState uint8

const (
	Low  PinState = 0
	High PinState = 1
)

type PinMode uint8

const (
	Input          PinMode = 0
	InputListening PinMode = 1
	Output         PinMode = 2
)
