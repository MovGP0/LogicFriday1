/* 0044591d __FF_MSGBANNER */

/* Library Function - Single Match
    __FF_MSGBANNER
   
   Library: Visual Studio 2003 Release */

void __cdecl __FF_MSGBANNER(void)

{
  if ((DAT_0046c560 == 1) || ((DAT_0046c560 == 0 && (DAT_00451a44 == 1)))) {
    FUN_004457a6(0xfc);
    if (DAT_0046c808 != (code *)0x0) {
      (*DAT_0046c808)();
    }
    FUN_004457a6(0xff);
  }
  return;
}
