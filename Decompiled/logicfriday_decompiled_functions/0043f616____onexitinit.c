/* 0043f616 ___onexitinit */

/* Library Function - Single Match
    ___onexitinit
   
   Library: Visual Studio 2003 Release */

undefined4 ___onexitinit(void)

{
  DAT_0046cd48 = _malloc(0x80);
  if (DAT_0046cd48 == (undefined4 *)0x0) {
    return 0x18;
  }
  *DAT_0046cd48 = 0;
  DAT_0046cd44 = DAT_0046cd48;
  return 0;
}
