package main

import (
	"fmt"

	"simplegostruct/pkg/model"
)

func main() {
	item := model.ThingModel{
		Value: 42,
		Name:  "hello",
	}

	fmt.Printf("name=%s value=%d\n", item.Name, item.Value)
}
