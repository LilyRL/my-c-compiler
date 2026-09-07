int putchar(int c);

int put_n(int n) {
  putchar(n + 48);
  putchar(10);
  return 0;
}

int test(int a, int b, int c, int d, int e, int f, int g, int h, int i) {
  put_n(a);
  put_n(b);
  put_n(c);
  put_n(d);
  put_n(e);
  put_n(f);
  put_n(g);
  put_n(h);
  put_n(i);

  if (a != 1 || b != 2 || c != 3 || d != 4 || e != 5 || f != 6 || g != 7 ||
      h != 8 || i != 9) {
    return 1;
  } else {
    return 0;
  }
}

int main(void) { return test(1, 2, 3, 4, 5, 6, 7, 8, 9); }
