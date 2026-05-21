/* 00440585 _fgetpos */

/* Library Function - Single Match
    _fgetpos
   
   Library: Visual Studio 2003 Release */

int __cdecl _fgetpos(FILE *_File,fpos_t *_Pos)

{
  int iVar1;
  longlong lVar2;
  
  lVar2 = __ftelli64(_File);
  *_Pos = lVar2;
  iVar1 = -1;
  if (((uint)lVar2 & *(uint *)((int)_Pos + 4)) != 0xffffffff) {
    iVar1 = 0;
  }
  return iVar1;
}
