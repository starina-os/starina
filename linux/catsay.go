package main

import (
	"bufio"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"runtime"
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

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/plain")
		w.Header().Set("Server", runtime.Version())
		catArt := `
      ____________
     < Hello Web! >
      ------------
          /
  /\_/\  /
 ( o.o )
 \(___)
`
		fmt.Fprint(w, catArt)
	})

	fmt.Fprintln(os.Stderr, "Starting cat server on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}
