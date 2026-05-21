/* 00440f2f ___endstdio */

/* Library Function - Single Match
    ___endstdio
   
   Library: Visual Studio 2003 Release */

void ___endstdio(void)

{
  __flushall();
  if (DAT_0046c718 != '\0') {
    __fcloseall();
    return;
  }
  return;
}
