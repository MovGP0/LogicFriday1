/* 0044056d _fsetpos */

/* Library Function - Single Match
    _fsetpos
   
   Library: Visual Studio 2003 Release */

int __cdecl _fsetpos(FILE *_File,fpos_t *_Pos)

{
  int iVar1;
  int unaff_retaddr;
  
  iVar1 = __fseeki64(_File,(ulonglong)*(uint *)((int)_Pos + 4),unaff_retaddr);
  return iVar1;
}
