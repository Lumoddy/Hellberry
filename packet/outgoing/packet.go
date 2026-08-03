package outgoing

import (
	"encoding/binary"
	"hellberry/packet"
	"io"
)

type OutgoingPacket interface {
	isOutgoingPacket()
}

type Ping struct{}

func (Ping) isOutgoingPacket() {}

type GetConfig struct{}

func (GetConfig) isOutgoingPacket() {}

type GetPin struct{ id uint8 }

func (GetPin) isOutgoingPacket() {}

type GetPinMode struct{ id uint8 }

func (GetPinMode) isOutgoingPacket() {}

type SetPin struct {
	id    uint8
	state packet.PinState
}

func (SetPin) isOutgoingPacket() {}

type SetPinMode struct {
	id   uint8
	mode packet.PinMode
}

func (SetPinMode) isOutgoingPacket() {}

func Write(writer io.ByteWriter, packet OutgoingPacket, endian binary.ByteOrder) error {
	switch packet := packet.(type) {
	case Ping:
		return writer.WriteByte(0x00)
	case GetConfig:
		return writer.WriteByte(0x01)
	case GetPin:
		err := writer.WriteByte(0x02)
		if err != nil {
			return err
		}
		return writer.WriteByte(packet.id)
	case GetPinMode:
		err := writer.WriteByte(0x03)
		if err != nil {
			return err
		}
		return writer.WriteByte(packet.id)
	case SetPin:
		err := writer.WriteByte(0x04)
		if err != nil {
			return err
		}
		err = writer.WriteByte(packet.id)
		if err != nil {
			return err
		}
		return writer.WriteByte(uint8(packet.state))
	case SetPinMode:
		err := writer.WriteByte(0x05)
		if err != nil {
			return err
		}
		err = writer.WriteByte(packet.id)
		if err != nil {
			return err
		}
		return writer.WriteByte(uint8(packet.mode))
	default:
		return nil
	}
}
