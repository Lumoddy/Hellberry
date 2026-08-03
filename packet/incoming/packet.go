package incoming

import (
	"encoding/binary"
	"hellberry/packet"
	"io"
	"strings"
)

type IncomingPacket interface {
	isIncomingPacket()
}

type Ok struct{}

func (Ok) isIncomingPacket() {}

type WholeConfig struct {
	name string
	pins []struct {
		id        uint8
		name      string
		readable  bool
		writeable bool
	}
}

func (WholeConfig) isIncomingPacket() {}

type GetPinResponse struct{ state packet.PinState }

func (GetPinResponse) isIncomingPacket() {}

type GetPinModeResponse struct{ mode packet.PinMode }

func (GetPinModeResponse) isIncomingPacket() {}

type PinListenChange struct {
	id    uint8
	state packet.PinState
}

func (PinListenChange) isIncomingPacket() {}

type BadPacket struct{}

func (BadPacket) isIncomingPacket() {}

type Panic struct{}

func (Panic) isIncomingPacket() {}

type Timeout struct{}

func (Timeout) isIncomingPacket() {}

type InvalidPinId struct{}

func (InvalidPinId) isIncomingPacket() {}

type InvalidPinMode struct{}

func (InvalidPinMode) isIncomingPacket() {}

type InvalidPacketTagError struct{}

func (InvalidPacketTagError) Error() string {
	return "invalid packet tag"
}

func Read(reader io.ByteReader, endian binary.ByteOrder) (result IncomingPacket, err error) {
	var tag byte
	tag, err = reader.ReadByte()
	if err != nil {
		return
	}

	switch tag {
	case 0:
		result = Ok{}
		return
	case 1:
		var name string
		name, err = parseString(reader, endian)
		if err != nil {
			return
		}

		var len uint16
		len, err = parseLen(reader, endian)
		if err != nil {
			return
		}

		pins := []struct {
			id        uint8
			name      string
			readable  bool
			writeable bool
		}{}

		for range len {
			var id uint8
			id, err = parseUint8(reader, endian)
			if err != nil {
				return
			}

			var name string
			name, err = parseString(reader, endian)
			if err != nil {
				return
			}

			var flags uint8
			flags, err = parseUint8(reader, endian)
			if err != nil {
				return
			}

			pins = append(pins, struct {
				id        uint8
				name      string
				readable  bool
				writeable bool
			}{
				id:        id,
				name:      name,
				readable:  flags&0x1 != 0,
				writeable: flags&0x2 != 0,
			})
		}

		return WholeConfig{name, pins}, nil
	case 2:
		result = Ok{}
		return
	case 3:
		var state uint8
		state, err = parseUint8(reader, endian)
		if err != nil {
			return
		}

		result = GetPinResponse{state: packet.PinState(state)}
		return
	case 4:
		var mode uint8
		mode, err = parseUint8(reader, endian)
		if err != nil {
			return
		}

		result = GetPinModeResponse{mode: packet.PinMode(mode)}
		return
	case 5:
		var id uint8
		id, err = parseUint8(reader, endian)
		if err != nil {
			return
		}

		var state uint8
		state, err = parseUint8(reader, endian)
		if err != nil {
			return
		}

		result = PinListenChange{id: id, state: packet.PinState(state)}
		return
	case 100:
		result = BadPacket{}
		return
	case 101:
		result = Panic{}
		return
	case 102:
		result = Timeout{}
		return
	case 103:
		result = InvalidPinId{}
		return
	case 104:
		result = InvalidPinMode{}
		return
	default:
		err = InvalidPacketTagError{}
		return
	}
}

func parseUint8(reader io.ByteReader, _ binary.ByteOrder) (value uint8, err error) {
	return reader.ReadByte()
}

func parseUint16(reader io.ByteReader, endian binary.ByteOrder) (value uint16, err error) {
	lenBytes := []byte{0, 0}

	for i := range 2 {
		var byte byte
		byte, err = reader.ReadByte()
		if err != nil {
			return
		} else {
			lenBytes[i] = byte
		}
	}

	value = endian.Uint16(lenBytes)
	return
}

func parseLen(reader io.ByteReader, endian binary.ByteOrder) (value uint16, err error) {
	return parseUint16(reader, endian)
}

func parseString(reader io.ByteReader, endian binary.ByteOrder) (string string, err error) {
	var len uint16
	len, err = parseLen(reader, endian)
	if err != nil {
		return
	}

	var builder strings.Builder
	builder.Grow(int(len))

	for range len {
		var byte byte
		byte, err = reader.ReadByte()
		if err != nil {
			return
		} else {
			builder.WriteByte(byte)
		}
	}

	string = builder.String()
	return
}
