package hellberry

import (
	"fmt"

	"go.bug.st/serial"
)

func main() {

	ports, err := serial.GetPortsList()

	if err != nil {
		fmt.Printf("err: %v\n", err.Error())
		return
	}

	fmt.Printf("ports: %v\n", ports)
}
