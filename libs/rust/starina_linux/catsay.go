package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"
)

func main() {
	reader := bufio.NewReader(os.Stdin)
	messageBytes, _ := io.ReadAll(reader)
	message := strings.TrimSpace(string(messageBytes))

	width := len(message) + 2
	fmt.Println()
	fmt.Printf("      %s\n", strings.Repeat("_", width))
	fmt.Printf("     < %s >\n", message)
	fmt.Printf("      %s\n", strings.Repeat("-", width))
	fmt.Println("          /")
	fmt.Println("  /\\_/\\  /")
	fmt.Println(" ( o.o )")
	fmt.Println(" \\(___)")
}
