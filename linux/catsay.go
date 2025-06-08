package main

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"os"
	"strings"
)

func main() {
	// DNS query
	ips, err := net.LookupIP("seiya.me")
	if err != nil {
		fmt.Printf("DNS lookup failed: %v\n", err)
	} else {
		fmt.Printf("seiya.me resolves to: %v\n", ips)
	}

	// Original catsay functionality
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
