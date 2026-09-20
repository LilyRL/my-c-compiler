int putchar(int c);

int put_char(int c) {
  putchar(c);
  return 0;
}

int put_digit(int d) {
  putchar(d + 48);
  return 0;
}

int put_newline(void) {
  putchar(10);
  return 0;
}

int print_int(int n) {
  if (n < 0) {
    putchar(45);
    n = -n;
  }

  if (n >= 10) {
    print_int(n / 10);
  }

  put_digit(n % 10);

  return 0;
}

int print_long(long n) {
  if (n < 0) {
    putchar(45);
    n = -n;
  }

  if (n >= 10) {
    print_long(n / 10);
  }

  put_digit(n % 10);

  return 0;
}
