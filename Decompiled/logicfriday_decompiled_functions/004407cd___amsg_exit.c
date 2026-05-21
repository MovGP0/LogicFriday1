/* 004407cd __amsg_exit */

/* Library Function - Single Match
    __amsg_exit
   
   Library: Visual Studio 2003 Release */

void __cdecl __amsg_exit(int param_1)

{
  if (DAT_0046c560 == 1) {
    __FF_MSGBANNER();
  }
  FUN_004457a6(param_1);
  (*(code *)PTR___exit_00451a40)(0xff);
  return;
}
