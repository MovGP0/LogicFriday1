/* 00441d6a __lock */

/* Library Function - Single Match
    __lock
   
   Library: Visual Studio 2003 Release */

void __cdecl __lock(int _File)

{
  int iVar1;
  
  if ((&DAT_00451e40)[_File * 2] == 0) {
    iVar1 = FUN_00441ceb(_File);
    if (iVar1 == 0) {
      __amsg_exit(0x11);
    }
  }
  EnterCriticalSection((LPCRITICAL_SECTION)(&DAT_00451e40)[_File * 2]);
  return;
}
