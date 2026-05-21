/* 00447fe5 _abort */

/* Library Function - Single Match
    _abort
   
   Library: Visual Studio 2003 Release */

void __cdecl _abort(void)

{
  FUN_004457a6(10);
  _raise(0x16);
                    /* WARNING: Subroutine does not return */
  __exit(3);
}
