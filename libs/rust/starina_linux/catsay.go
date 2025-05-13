package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func main() {
	// Write to /virtfs/stdout
	fmt.Fprintf(os.Stderr, "writing to /virtfs/stdout\n")
	stdout, err := os.OpenFile("/virtfs/stdout", os.O_WRONLY, 0644)
	if err != nil {
		panic(err)
	}
	defer stdout.Close()
	stdout.WriteString("hello from catsay\n")

	fmt.Fprintln(os.Stderr, "reading from stdin")
	reader := bufio.NewReader(os.Stdin)
	message, _ := reader.ReadString('\n')
	message = strings.TrimSpace(message)

	fmt.Fprintln(os.Stderr, "writing to stdout")
	width := len(message) + 2
	fmt.Printf(" %s\n", strings.Repeat("_", width))
	fmt.Printf("< %s >\n", message)
	fmt.Printf(" %s\n", strings.Repeat("-", width))
	fmt.Println("  /\\_/\\")
	fmt.Println(" (owo)")
	fmt.Println("  >^<")
}
