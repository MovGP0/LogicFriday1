/* 00444ea5 __fseeki64 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __fseeki64
   
   Library: Visual Studio 2003 Release */

int __cdecl __fseeki64(FILE *_File,longlong _Offset,int _Origin)

{
  int iVar1;
  undefined4 in_stack_00000008;
  
  __lock_file(_File);
  iVar1 = __fseeki64_lk(_File,in_stack_00000008,(undefined4)_Offset,_Offset._4_4_);
  FUN_00444ee7();
  return iVar1;
}
