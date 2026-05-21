/* 00445086 __ftelli64 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __ftelli64
   
   Library: Visual Studio 2003 Release */

longlong __cdecl __ftelli64(FILE *_File)

{
  ulonglong uVar1;
  
  __lock_file(_File);
  uVar1 = __ftelli64_lk((uint *)_File);
  FUN_004450c3();
  return uVar1;
}
