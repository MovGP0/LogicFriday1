/* 00440f95 __unlock_file */

/* Library Function - Single Match
    __unlock_file
   
   Libraries: Visual Studio 2003 Release, Visual Studio 2005 Release */

void __cdecl __unlock_file(FILE *_File)

{
  if (((FILE *)((int)&DAT_00451a44 + 3U) < _File) && (_File < (FILE *)0x451ca9)) {
    FUN_00441cd6(((int)&_File[-0x228d3]._bufsiz >> 5) + 0x10);
    return;
  }
  LeaveCriticalSection((LPCRITICAL_SECTION)(_File + 1));
  return;
}
